//! Board discovery service.

use regex::Regex;
use reqwest::blocking::Client;
use scraper::{Html, Selector};

use std::collections::{HashMap, HashSet};

use crate::models::{
    Board, BoardDiscoveryResult, Config, DiscoveryConfig, KeywordMapping, ManualReviewItem,
};
use crate::services::SelectorDetector;
use crate::utils::http::fetch_page_with_timeout;
use crate::utils::url;

/// Service for discovering boards on department websites
pub struct BoardDiscoveryService<'a> {
    client: &'a Client,
    keywords: Vec<KeywordMapping>,
    selector_detector: SelectorDetector,
    config: DiscoveryConfig,
    sitemap_timeout: u64,
}

impl<'a> BoardDiscoveryService<'a> {
    /// Create a new board discovery service
    pub fn new(
        client: &'a Client,
        keywords: Vec<KeywordMapping>,
        selector_detector: SelectorDetector,
        config: &Config,
    ) -> Self {
        Self {
            client,
            keywords,
            selector_detector,
            config: config.discovery.clone(),
            sitemap_timeout: config.http.sitemap_timeout_secs,
        }
    }

    /// Discover boards for a department
    pub fn discover(&self, campus: &str, dept_name: &str, dept_url: &str) -> BoardDiscoveryResult {
        let mut result = BoardDiscoveryResult::default();

        // Validate URL
        if dept_url == "NOT_FOUND" || !dept_url.starts_with("http") {
            result.manual_review = Some(ManualReviewItem {
                campus: campus.to_string(),
                name: dept_name.to_string(),
                url: dept_url.to_string(),
                reason: "Homepage URL is invalid".to_string(),
            });
            return result;
        }

        // Fetch department homepage
        let document =
            match fetch_page_with_timeout(self.client, dept_url, self.sitemap_timeout + 2) {
                Ok(doc) => doc,
                Err(e) => {
                    result.manual_review = Some(ManualReviewItem {
                        campus: campus.to_string(),
                        name: dept_name.to_string(),
                        url: dept_url.to_string(),
                        reason: format!("Failed to fetch homepage: {}", e),
                    });
                    return result;
                }
            };

        println!("    Accessed: {}", dept_url);

        // Detect default CMS selectors
        let default_selectors = self.selector_detector.detect(&document, dept_url);

        // Try sitemap first
        if let Some(sitemap_doc) = self.find_sitemap(&document, dept_url) {
            let boards = self.extract_boards(&sitemap_doc, dept_url, &default_selectors);
            if !boards.is_empty() {
                result.boards = boards;
                return result;
            }
            println!("    Sitemap yielded no results, falling back to homepage");
        }

        // Fall back to homepage
        result.boards = self.extract_boards(&document, dept_url, &default_selectors);

        result
    }

    /// Try to find and fetch the sitemap page
    fn find_sitemap(&self, document: &Html, base_url: &str) -> Option<Html> {
        let link_selector = Selector::parse("a").unwrap();
        let sitemap_pattern = Regex::new(r"(?i)사이트맵|sitemap").unwrap();

        for element in document.select(&link_selector) {
            let text: String = element.text().collect();
            if sitemap_pattern.is_match(&text) {
                if let Some(href) = element.value().attr("href") {
                    let sitemap_url = url::resolve(base_url, href);
                    if let Ok(sitemap_doc) =
                        fetch_page_with_timeout(self.client, &sitemap_url, self.sitemap_timeout)
                    {
                        println!("    Found sitemap: {}", sitemap_url);
                        return Some(sitemap_doc);
                    }
                }
            }
        }

        None
    }

    /// Check if a link is likely a valid board link
    fn is_valid_board_link(&self, text: &str, href: &str) -> bool {
        // Check blacklist patterns
        for pattern in &self.config.blacklist_patterns {
            if href.contains(pattern) {
                return false;
            }
        }

        // Long text is likely a notice title, not a board name
        if text.chars().count() > self.config.max_board_name_length {
            return false;
        }

        true
    }

    /// Extract boards from an HTML document
    fn extract_boards(
        &self,
        document: &Html,
        base_url: &str,
        default_selectors: &Option<crate::models::CmsSelectors>,
    ) -> Vec<Board> {
        let mut boards = Vec::new();
        let mut seen_urls = HashSet::new();
        let mut id_counts: HashMap<String, usize> = HashMap::new();

        let base_domain = url::get_domain(base_url);
        let link_selector = Selector::parse("a[href]").unwrap();

        for element in document.select(&link_selector) {
            let text: String = element.text().collect::<String>().trim().to_string();
            let href = match element.value().attr("href") {
                Some(h) => h,
                None => continue,
            };

            if !self.is_valid_board_link(&text, href) {
                continue;
            }

            let full_url = url::resolve(base_url, href);

            // Skip invalid URLs
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
            for mapping in &self.keywords {
                if !text.contains(&mapping.keyword) {
                    continue;
                }

                // Try to detect CMS selectors
                let selectors = self
                    .selector_detector
                    .detect(document, &full_url)
                    .or_else(|| default_selectors.clone());

                let selectors = match selectors {
                    Some(s) => s,
                    None => continue,
                };

                // Generate unique ID
                let base_id = mapping.id.clone();
                let count = id_counts.entry(base_id.clone()).or_insert(0);
                *count += 1;

                let final_id = if *count > 1 {
                    format!("{}_{}", base_id, count)
                } else {
                    base_id
                };

                let board_name = if text.is_empty() {
                    mapping.display_name.clone()
                } else {
                    text.clone()
                };

                boards.push(Board {
                    id: final_id,
                    name: board_name,
                    url: full_url.clone(),
                    selectors,
                });

                seen_urls.insert(full_url.clone());
                break;
            }
        }

        boards
    }
}
