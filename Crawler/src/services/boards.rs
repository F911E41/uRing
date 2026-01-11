// src/services/boards.rs

//! Board discovery service.
//!
//! Discovers notice boards on department websites by analyzing page content
//! and matching against known keywords.

use std::collections::{HashMap, HashSet};

use regex::Regex;
use reqwest::blocking::Client;
use scraper::{Html, Selector};

use crate::error::Result;
use crate::models::{
    Board, BoardDiscoveryResult, CmsSelectors, DiscoveryConfig, KeywordMapping, ManualReviewItem,
};
use crate::services::SelectorDetector;
use crate::utils::{http::fetch_page_with_timeout, log, url};

/// Service for discovering boards on department websites.
pub struct BoardDiscoveryService<'a> {
    client: &'a Client,
    keywords: Vec<KeywordMapping>,
    selector_detector: SelectorDetector,
    config: DiscoveryConfig,
    sitemap_timeout: u64,
}

impl<'a> BoardDiscoveryService<'a> {
    /// Create a new board discovery service.
    pub fn new(
        client: &'a Client,
        keywords: Vec<KeywordMapping>,
        selector_detector: SelectorDetector,
        config: &DiscoveryConfig,
        sitemap_timeout: u64,
    ) -> Self {
        Self {
            client,
            keywords,
            selector_detector,
            config: config.clone(),
            sitemap_timeout,
        }
    }

    /// Discover boards for a department.
    pub fn discover(&self, campus: &str, dept_name: &str, dept_url: &str) -> BoardDiscoveryResult {
        let mut result = BoardDiscoveryResult::default();

        // Validate URL
        if !Self::is_valid_url(dept_url) {
            result.manual_review = Some(ManualReviewItem {
                campus: campus.to_string(),
                name: dept_name.to_string(),
                url: dept_url.to_string(),
                reason: "Homepage URL is invalid".to_string(),
            });
            return result;
        }

        // Fetch department homepage
        let document = match self.fetch_department_page(dept_url) {
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

        log::info(&format!("    Accessed: {dept_url}"));

        // Detect default CMS selectors
        let default_selectors = self.selector_detector.detect(&document, dept_url);

        // Try sitemap first, then fall back to homepage
        let source_doc = self
            .find_sitemap(&document, dept_url)
            .unwrap_or_else(|| document.clone());

        result.boards = self.extract_boards(&source_doc, dept_url, &default_selectors);

        if result.boards.is_empty() && source_doc.html() != document.html() {
            // Sitemap didn't work, try homepage
            log::info("    Sitemap yielded no results, falling back to homepage");
            result.boards = self.extract_boards(&document, dept_url, &default_selectors);
        }

        result
    }

    fn is_valid_url(url: &str) -> bool {
        url != "NOT_FOUND" && url.starts_with("http")
    }

    fn fetch_department_page(&self, url: &str) -> Result<Html> {
        fetch_page_with_timeout(self.client, url, self.sitemap_timeout + 2)
    }

    /// Try to find and fetch the sitemap page.
    fn find_sitemap(&self, document: &Html, base_url: &str) -> Option<Html> {
        let link_selector = Selector::parse("a").ok()?;
        let sitemap_pattern = Regex::new(r"(?i)사이트맵|sitemap").ok()?;

        for element in document.select(&link_selector) {
            let text: String = element.text().collect();
            if !sitemap_pattern.is_match(&text) {
                continue;
            }

            if let Some(href) = element.value().attr("href") {
                let sitemap_url = url::resolve(base_url, href);
                if let Ok(sitemap_doc) =
                    fetch_page_with_timeout(self.client, &sitemap_url, self.sitemap_timeout)
                {
                    log::info(&format!("    Found sitemap: {sitemap_url}"));
                    return Some(sitemap_doc);
                }
            }
        }

        None
    }

    /// Check if a link is likely a valid board link.
    fn is_valid_board_link(&self, text: &str, href: &str) -> bool {
        // Check blacklist patterns
        if self
            .config
            .blacklist_patterns
            .iter()
            .any(|p| href.contains(p))
        {
            return false;
        }

        // Long text is likely a notice title, not a board name
        text.chars().count() <= self.config.max_board_name_length
    }

    /// Extract boards from an HTML document.
    fn extract_boards(
        &self,
        document: &Html,
        base_url: &str,
        default_selectors: &Option<CmsSelectors>,
    ) -> Vec<Board> {
        let mut boards = Vec::new();
        let mut seen_urls = HashSet::new();
        let mut id_counts: HashMap<String, usize> = HashMap::new();

        let base_domain = url::get_domain(base_url);
        let link_selector = match Selector::parse("a[href]") {
            Ok(s) => s,
            Err(_) => return boards,
        };

        for element in document.select(&link_selector) {
            let text: String = element.text().collect::<String>().trim().to_string();
            let Some(href) = element.value().attr("href") else {
                continue;
            };

            if !self.is_valid_board_link(&text, href) {
                continue;
            }

            let full_url = url::resolve(base_url, href);

            // Skip already seen, javascript links, or anchors
            if seen_urls.contains(&full_url) || href.contains("javascript") || href == "#" {
                continue;
            }

            // Skip external links
            if let (Some(base_dom), Some(link_dom)) = (&base_domain, url::get_domain(&full_url)) {
                if base_dom != &link_dom {
                    continue;
                }
            }

            // Match against keywords
            if let Some(board) = self.try_create_board(
                &text,
                &full_url,
                document,
                default_selectors,
                &mut id_counts,
            ) {
                seen_urls.insert(full_url);
                boards.push(board);
            }
        }

        boards
    }

    fn try_create_board(
        &self,
        text: &str,
        url: &str,
        _document: &Html,
        default_selectors: &Option<CmsSelectors>,
        id_counts: &mut HashMap<String, usize>,
    ) -> Option<Board> {
        // Find matching keyword
        let mapping = self.keywords.iter().find(|m| text.contains(&m.keyword))?;

        // Try to detect CMS selectors by fetching the actual board page
        let selectors = self.detect_board_selectors(url, default_selectors)?;

        // Generate unique ID
        let count = id_counts.entry(mapping.id.clone()).or_insert(0);
        *count += 1;

        let final_id = if *count > 1 {
            format!("{}_{}", mapping.id, count)
        } else {
            mapping.id.clone()
        };

        let board_name = if text.is_empty() {
            mapping.display_name.clone()
        } else {
            text.to_string()
        };

        Some(Board {
            id: final_id,
            name: board_name,
            url: url.to_string(),
            selectors,
        })
    }

    /// Detect CMS selectors by fetching the board page and analyzing its structure.
    fn detect_board_selectors(
        &self,
        url: &str,
        default_selectors: &Option<CmsSelectors>,
    ) -> Option<CmsSelectors> {
        // First try using default selectors if available
        if let Some(selectors) = default_selectors {
            return Some(selectors.clone());
        }

        // Fetch the actual board page to detect CMS
        match fetch_page_with_timeout(self.client, url, self.sitemap_timeout) {
            Ok(board_doc) => {
                if let Some(selectors) = self.selector_detector.detect(&board_doc, url) {
                    return Some(selectors);
                }
            }
            Err(_) => {
                // Failed to fetch, try with fallback selectors
            }
        }

        // Return fallback selectors for common patterns
        Some(CmsSelectors::fallback())
    }
}
