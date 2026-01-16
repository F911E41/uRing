// src/models/selectors.rs

//! CSS selectors for scraping a notice board.

use serde::{Deserialize, Serialize};

/// CSS selectors for scraping a notice board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsSelectors {
    /// Selector for each row/item in the notice list
    pub row_selector: String,

    /// Selector for the title element within a row
    pub title_selector: String,

    /// Selector for the date element within a row
    pub date_selector: String,

    /// Selector for the author element within a row
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_selector: Option<String>,

    /// Selector for the notice body content on the detail page
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_selector: Option<String>,

    /// HTML attribute name for extracting links (usually "href")
    #[serde(default = "default_attr_name")]
    pub attr_name: String,

    /// Optional selector for the link element (if different from title)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_selector: Option<String>,
}

fn default_attr_name() -> String {
    "href".to_string()
}

impl Default for CmsSelectors {
    fn default() -> Self {
        Self {
            row_selector: "tr".to_string(),
            title_selector: "a".to_string(),
            date_selector: "td:last-child".to_string(),
            author_selector: None,
            body_selector: None,
            attr_name: default_attr_name(),
            link_selector: None,
        }
    }
}

impl CmsSelectors {
    /// Create selectors from a CMS pattern.
    pub fn from_pattern(
        row: impl Into<String>,
        title: impl Into<String>,
        date: impl Into<String>,
        attr: impl Into<String>,
    ) -> Self {
        Self {
            row_selector: row.into(),
            title_selector: title.into(),
            date_selector: date.into(),
            author_selector: None,
            body_selector: None,
            attr_name: attr.into(),
            link_selector: None,
        }
    }

    /// Create fallback selectors that work with common board patterns.
    /// These are generic selectors that should work with most table-based boards.
    pub fn fallback() -> Self {
        Self {
            row_selector: "table tr:has(a)".to_string(),
            title_selector: "a".to_string(),
            date_selector: "td:last-child".to_string(),
            author_selector: None,
            body_selector: None,
            attr_name: "href".to_string(),
            link_selector: None,
        }
    }
}
