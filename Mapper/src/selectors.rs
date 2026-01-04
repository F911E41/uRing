//! CMS detection and selector utilities.

use scraper::Html;
use serde::{Deserialize, Serialize};

/// CSS selectors for scraping a board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsSelectors {
    pub row_selector: String,
    pub title_selector: String,
    pub date_selector: String,
    pub attr_name: String,
}

impl CmsSelectors {
    /// Standard Yonsei CMS selectors
    fn yonsei_cms() -> Self {
        Self {
            row_selector: "tr:has(a.c-board-title)".to_string(),
            title_selector: "a.c-board-title".to_string(),
            date_selector: "td:nth-last-child(1)".to_string(),
            attr_name: "href".to_string(),
        }
    }

    /// XE board system selectors
    fn xe_board() -> Self {
        Self {
            row_selector: "li.xe-list-board-list--item:not(.xe-list-board-list--header)"
                .to_string(),
            title_selector: "a.xe-list-board-list__title-link".to_string(),
            date_selector: ".xe-list-board-list__created_at".to_string(),
            attr_name: "href".to_string(),
        }
    }
}

/// Detect CMS type and return appropriate selectors
pub fn detect_cms_selectors(document: &Html, url: &str) -> Option<CmsSelectors> {
    let html_str = document.html().to_lowercase();

    // Standard Yonsei CMS
    if url.contains(".do") || html_str.contains("c-board-title") {
        return Some(CmsSelectors::yonsei_cms());
    }

    // XE board system
    if html_str.contains("xe-list-board") || html_str.contains("xe_board") {
        return Some(CmsSelectors::xe_board());
    }

    None
}
