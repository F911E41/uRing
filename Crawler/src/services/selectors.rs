//! CMS selector detection service.
//!
//! Detects the CMS type used by a website and returns appropriate CSS selectors.

use scraper::Html;

use crate::models::{CmsPattern, CmsSelectors, Seed};

/// Service for detecting CMS types and returning appropriate selectors.
pub struct SelectorDetector {
    patterns: Vec<CmsPattern>,
}

impl SelectorDetector {
    /// Create a new selector detector with the given patterns.
    pub fn new(patterns: Vec<CmsPattern>) -> Self {
        Self { patterns }
    }

    /// Detect CMS type and return appropriate selectors.
    /// Returns the selectors and the matched pattern name.
    pub fn detect(&self, document: &Html, url: &str) -> Option<CmsSelectors> {
        let html_lower = document.html().to_lowercase();

        self.patterns.iter().find_map(|pattern| {
            if self.matches_pattern(pattern, url, &html_lower) {
                log::debug!("Detected CMS pattern: '{}' for URL: {}", pattern.name, url);
                Some(CmsSelectors::from_pattern(
                    &pattern.row_selector,
                    &pattern.title_selector,
                    &pattern.date_selector,
                    &pattern.link_attr,
                ))
            } else {
                None
            }
        })
    }

    fn matches_pattern(&self, pattern: &CmsPattern, url: &str, html_lower: &str) -> bool {
        // Check URL pattern
        if let Some(url_pattern) = &pattern.detect_url_contains {
            if url.contains(url_pattern) {
                return true;
            }
        }

        // Check HTML pattern
        if let Some(html_pattern) = &pattern.detect_html_contains {
            if html_lower.contains(&html_pattern.to_lowercase()) {
                return true;
            }
        }

        false
    }
}

impl Default for SelectorDetector {
    fn default() -> Self {
        Self::new(Seed::default().cms_patterns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_detector() {
        let detector = SelectorDetector::default();
        assert!(!detector.patterns.is_empty());
    }
}
