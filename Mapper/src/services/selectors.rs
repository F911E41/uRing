//! CMS selector detection service.

use scraper::Html;

use crate::models::CmsPattern;
use crate::models::CmsSelectors;

/// Service for detecting CMS types and returning appropriate selectors
pub struct SelectorDetector {
    patterns: Vec<CmsPattern>,
}

impl SelectorDetector {
    /// Create a new selector detector with the given patterns
    pub fn new(patterns: Vec<CmsPattern>) -> Self {
        Self { patterns }
    }

    /// Detect CMS type and return appropriate selectors
    pub fn detect(&self, document: &Html, url: &str) -> Option<CmsSelectors> {
        let html_str = document.html().to_lowercase();

        for pattern in &self.patterns {
            let mut matched = false;

            // Check URL pattern
            if let Some(url_pattern) = &pattern.detect_url_contains {
                if url.contains(url_pattern) {
                    matched = true;
                }
            }

            // Check HTML pattern
            if let Some(html_pattern) = &pattern.detect_html_contains {
                if html_str.contains(&html_pattern.to_lowercase()) {
                    matched = true;
                }
            }

            if matched {
                return Some(CmsSelectors {
                    row_selector: pattern.row_selector.clone(),
                    title_selector: pattern.title_selector.clone(),
                    date_selector: pattern.date_selector.clone(),
                    attr_name: pattern.link_attr.clone(),
                });
            }
        }

        None
    }
}

impl Default for SelectorDetector {
    fn default() -> Self {
        use crate::models::Seed;
        Self::new(Seed::default().cms_patterns)
    }
}
