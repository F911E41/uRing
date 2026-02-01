//! Seed data model structures (campuses, keywords, CMS patterns).

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Root seed data structure containing initial configuration for discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seed {
    /// List of campuses to crawl
    pub campuses: Vec<CampusInfo>,

    /// Board keyword to ID mappings
    pub keywords: Vec<KeywordMapping>,

    /// CMS detection patterns and selectors
    #[serde(default)]
    pub cms_patterns: Vec<CmsPattern>,
}

impl Seed {
    /// Load seed data from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Validate that seed data contains required fields.
    pub fn validate(&self) -> Result<()> {
        if self.campuses.is_empty() {
            return Err(crate::error::AppError::validation(
                "No campuses defined in seed data",
            ));
        }
        if self.keywords.is_empty() {
            return Err(crate::error::AppError::validation(
                "No keywords defined in seed data",
            ));
        }
        Ok(())
    }
}

/// Campus information for initial discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampusInfo {
    /// Campus name (e.g., "신촌캠퍼스")
    pub name: String,

    /// URL of the campus department listing page
    pub url: String,
}

/// Mapping from board keyword to standardized ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordMapping {
    /// Keyword to search for in link text
    pub keyword: String,

    /// Standardized ID for the board type
    pub id: String,

    /// Human-readable display name
    pub display_name: String,
}

/// CMS detection pattern with corresponding selectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsPattern {
    /// Pattern name for identification
    pub name: String,

    /// URL substring to match
    #[serde(default)]
    pub detect_url_contains: Option<String>,

    /// HTML content substring to match
    #[serde(default)]
    pub detect_html_contains: Option<String>,

    /// CSS selector for notice rows
    pub row_selector: String,

    /// CSS selector for title element
    pub title_selector: String,

    /// CSS selector for date element
    pub date_selector: String,

    /// HTML attribute for link extraction
    pub link_attr: String,
}

impl Default for Seed {
    fn default() -> Self {
        Self {
            campuses: vec![
                CampusInfo {
                    name: "신촌캠퍼스".to_string(),
                    url: "https://www.yonsei.ac.kr/sc/186/subview.do".to_string(),
                },
                CampusInfo {
                    name: "미래캠퍼스".to_string(),
                    url: "https://mirae.yonsei.ac.kr/wj/1413/subview.do".to_string(),
                },
            ],
            keywords: vec![
                KeywordMapping {
                    keyword: "학부공지".to_string(),
                    id: "academic".to_string(),
                    display_name: "학사공지".to_string(),
                },
                KeywordMapping {
                    keyword: "학사공지".to_string(),
                    id: "academic".to_string(),
                    display_name: "학사공지".to_string(),
                },
                KeywordMapping {
                    keyword: "대학원공지".to_string(),
                    id: "grad_notice".to_string(),
                    display_name: "대학원공지".to_string(),
                },
                KeywordMapping {
                    keyword: "장학".to_string(),
                    id: "scholarship".to_string(),
                    display_name: "장학공지".to_string(),
                },
                KeywordMapping {
                    keyword: "취업".to_string(),
                    id: "career".to_string(),
                    display_name: "취업/진로".to_string(),
                },
                KeywordMapping {
                    keyword: "공지사항".to_string(),
                    id: "notice".to_string(),
                    display_name: "일반공지".to_string(),
                },
            ],
            cms_patterns: Self::default_patterns(),
        }
    }
}

impl Seed {
    fn default_patterns() -> Vec<CmsPattern> {
        vec![
            CmsPattern {
                name: "yonsei_standard".to_string(),
                detect_url_contains: Some(".do".to_string()),
                detect_html_contains: Some("c-board-title".to_string()),
                row_selector: "tr:has(a.c-board-title)".to_string(),
                title_selector: "a.c-board-title".to_string(),
                date_selector: "td:nth-last-child(1)".to_string(),
                link_attr: "href".to_string(),
            },
            CmsPattern {
                name: "xe_board".to_string(),
                detect_url_contains: None,
                detect_html_contains: Some("xe-list-board".to_string()),
                row_selector: "li.xe-list-board-list--item:not(.xe-list-board-list--header)"
                    .to_string(),
                title_selector: "a.xe-list-board-list__title-link".to_string(),
                date_selector: ".xe-list-board-list__created_at".to_string(),
                link_attr: "href".to_string(),
            },
        ]
    }
}
