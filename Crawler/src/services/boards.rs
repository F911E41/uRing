//! Board discovery service.
//!
//! Discovers notice boards on department websites by analyzing page content
//! and matching against known keywords.

use std::collections::{HashMap, HashSet};

use futures::future;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};

use crate::error::Result;
use crate::models::{
    Board, BoardDiscoveryResult, CmsSelectors, DiscoveryConfig, KeywordMapping, ManualReviewItem,
};
use crate::services::SelectorDetector;
use crate::utils::{get_domain, http::fetch_page_async, resolve};

/// Service for discovering boards on department websites.
pub struct BoardDiscoveryService<'a> {
    client: &'a Client,
    keywords: Vec<KeywordMapping>,
    selector_detector: SelectorDetector,
    config: DiscoveryConfig,
}

impl<'a> BoardDiscoveryService<'a> {
    /// Create a new board discovery service.
    pub fn new(
        client: &'a Client,
        keywords: Vec<KeywordMapping>,
        selector_detector: SelectorDetector,
        config: &DiscoveryConfig,
    ) -> Self {
        Self {
            client,
            keywords,
            selector_detector,
            config: config.clone(),
        }
    }

    /// Discover boards for a department.
    pub async fn discover(
        &self,
        campus: &str,
        dept_name: &str,
        dept_url: &str,
    ) -> BoardDiscoveryResult {
        let mut result = BoardDiscoveryResult::default();

        if !Self::is_valid_url(dept_url) {
            result.manual_review = Some(ManualReviewItem {
                campus: campus.to_string(),
                name: dept_name.to_string(),
                url: dept_url.to_string(),
                reason: "Homepage URL is invalid".to_string(),
            });
            return result;
        }

        let document = match self.fetch_department_page(dept_url).await {
            Ok(doc) => doc,
            Err(e) => {
                result.manual_review = Some(ManualReviewItem {
                    campus: campus.to_string(),
                    name: dept_name.to_string(),
                    url: dept_url.to_string(),
                    reason: format!("Failed to fetch homepage: {e}"),
                });
                return result;
            }
        };

        log::debug!("Accessed: {}", dept_url);

        let default_selectors = self.selector_detector.detect(&document, dept_url);

        // Extract boards from homepage
        let homepage_boards = self
            .extract_boards(&document, dept_url, &default_selectors)
            .await;

        // Try sitemap and merge results (instead of fallback-only)
        let sitemap_boards = if let Some(sitemap_doc) = self.find_sitemap(&document, dept_url).await
        {
            self.extract_boards(&sitemap_doc, dept_url, &default_selectors)
                .await
        } else {
            Vec::new()
        };

        // Merge boards from both sources, deduplicating by URL
        result.boards = Self::merge_boards(homepage_boards, sitemap_boards);

        // If no boards found at all, mark for manual review
        if result.boards.is_empty() {
            result.manual_review = Some(ManualReviewItem {
                campus: campus.to_string(),
                name: dept_name.to_string(),
                url: dept_url.to_string(),
                reason: "No boards discovered from homepage or sitemap".to_string(),
            });
        }

        result
    }

    fn is_valid_url(url: &str) -> bool {
        url != "NOT_FOUND" && url.starts_with("http")
    }

    /// Merge boards from multiple sources, deduplicating by URL.
    fn merge_boards(primary: Vec<Board>, secondary: Vec<Board>) -> Vec<Board> {
        use std::collections::HashSet;
        let mut seen_urls: HashSet<String> = HashSet::new();
        let mut merged = Vec::new();

        for board in primary {
            if seen_urls.insert(board.url.clone()) {
                merged.push(board);
            }
        }

        for board in secondary {
            if seen_urls.insert(board.url.clone()) {
                merged.push(board);
            }
        }

        merged
    }

    async fn fetch_department_page(&self, url: &str) -> Result<Html> {
        fetch_page_async(self.client, url).await
    }

    async fn find_sitemap(&self, document: &Html, base_url: &str) -> Option<Html> {
        let link_selector = Selector::parse("a").ok()?;
        let sitemap_pattern = Regex::new(r"(?i)사이트맵|sitemap").ok()?;

        for element in document.select(&link_selector) {
            let text: String = element.text().collect();
            if !sitemap_pattern.is_match(&text) {
                continue;
            }

            if let Some(href) = element.value().attr("href") {
                if let Some(sitemap_url) = resolve(base_url, href) {
                    if let Ok(sitemap_doc) = fetch_page_async(self.client, &sitemap_url).await {
                        log::debug!("Found sitemap: {}", sitemap_url);
                        return Some(sitemap_doc);
                    }
                }
            }
        }
        None
    }

    fn is_valid_board_link(&self, text: &str, href: &str) -> bool {
        if self
            .config
            .blacklist_patterns
            .iter()
            .any(|p| href.contains(p))
        {
            return false;
        }
        text.chars().count() <= self.config.max_board_name_length
    }

    async fn extract_boards(
        &self,
        document: &Html,
        base_url: &str,
        default_selectors: &Option<CmsSelectors>,
    ) -> Vec<Board> {
        let mut id_counts: HashMap<String, usize> = HashMap::new();
        let base_domain = get_domain(base_url);
        let link_selector = Selector::parse("a[href]").unwrap();

        let mut seen_urls = HashSet::new();
        let mut links_to_process = Vec::new();

        for element in document.select(&link_selector) {
            let text = element.text().collect::<String>().trim().to_string();
            if let Some(href) = element.value().attr("href") {
                if !self.is_valid_board_link(&text, href) {
                    continue;
                }
                let Some(full_url) = resolve(base_url, href) else {
                    continue;
                };
                if seen_urls.contains(&full_url) || href.contains("javascript") || href == "#" {
                    continue;
                }

                if let (Some(base_dom), Some(link_dom)) = (&base_domain, get_domain(&full_url)) {
                    if base_dom != &link_dom {
                        continue;
                    }
                }

                if seen_urls.insert(full_url.clone()) {
                    links_to_process.push((text, full_url));
                }
            }
        }

        let board_futures: Vec<_> = links_to_process
            .into_iter()
            .map(|(text, url)| self.try_create_board(text, url, default_selectors))
            .collect();

        let results: Vec<_> = future::join_all(board_futures).await;
        results
            .into_iter()
            .filter_map(|b| b)
            .fold(Vec::new(), |mut acc, mut board| {
                let count = id_counts.entry(board.id.clone()).or_insert(0);
                *count += 1;
                if *count > 1 {
                    board.id = format!("{}_{}", board.id, *count);
                }
                acc.push(board);
                acc
            })
    }

    async fn try_create_board(
        &self,
        text: String,
        url: String,
        default_selectors: &Option<CmsSelectors>,
    ) -> Option<Board> {
        let mapping = self.keywords.iter().find(|m| text.contains(&m.keyword))?;
        let selectors = self.detect_board_selectors(&url, default_selectors).await?;
        let board_name = if text.is_empty() {
            mapping.display_name.clone()
        } else {
            text
        };
        Some(Board {
            id: mapping.id.clone(),
            name: board_name,
            url,
            selectors,
        })
    }

    async fn detect_board_selectors(
        &self,
        url: &str,
        default_selectors: &Option<CmsSelectors>,
    ) -> Option<CmsSelectors> {
        if let Some(selectors) = default_selectors {
            return Some(selectors.clone());
        }

        if let Ok(board_doc) = fetch_page_async(self.client, url).await {
            if let Some(selectors) = self.selector_detector.detect(&board_doc, url) {
                return Some(selectors);
            }
        }

        Some(CmsSelectors::fallback())
    }
}
