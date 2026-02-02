//! Application configuration structures.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

/// Root application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP and crawling behavior settings
    #[serde(default)]
    pub crawler: CrawlerConfig,

    /// Board discovery rules
    #[serde(default)]
    pub discovery: DiscoveryConfig,

    /// Text preprocessing settings
    #[serde(default)]
    pub cleaning: CleaningConfig,

    /// Campus definitions
    #[serde(default)]
    pub campuses: Vec<CampusInfo>,

    /// Board keyword to ID mappings
    #[serde(default)]
    pub keywords: Vec<KeywordMapping>,

    /// CMS detection patterns and selectors
    #[serde(default)]
    pub cms_patterns: Vec<CmsPattern>,
}

impl Config {
    /// Load configuration from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Load configuration or return default if loading fails.
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        Self::load(&path).unwrap_or_else(|e| {
            log::warn!(
                "Config load failed from {:?}: {}. Using defaults.",
                path.as_ref(),
                e
            );
            Self::default()
        })
    }

    /// Validate configuration values for basic sanity.
    pub fn validate(&self) -> Result<()> {
        if self.crawler.user_agent.trim().is_empty() {
            return Err(AppError::validation("crawler.user_agent is empty"));
        }
        if self.crawler.timeout_secs == 0 {
            return Err(AppError::validation("crawler.timeout_secs must be > 0"));
        }
        if self.crawler.sitemap_timeout_secs == 0 {
            return Err(AppError::validation(
                "crawler.sitemap_timeout_secs must be > 0",
            ));
        }
        if self.crawler.max_concurrent == 0 {
            return Err(AppError::validation("crawler.max_concurrent must be > 0"));
        }
        if self.discovery.max_board_name_length == 0 {
            return Err(AppError::validation(
                "discovery.max_board_name_length must be > 0",
            ));
        }
        if self.campuses.is_empty() {
            return Err(AppError::validation("No campuses defined"));
        }
        if self.keywords.is_empty() {
            return Err(AppError::validation("No keywords defined"));
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            crawler: CrawlerConfig::default(),
            discovery: DiscoveryConfig::default(),
            cleaning: CleaningConfig::default(),
            campuses: defaults::default_campuses(),
            keywords: defaults::default_keywords(),
            cms_patterns: defaults::default_cms_patterns(),
        }
    }
}

/// HTTP client and crawling behavior settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlerConfig {
    /// User-Agent header for HTTP requests
    #[serde(default = "defaults::user_agent")]
    pub user_agent: String,

    /// Request timeout in seconds
    #[serde(default = "defaults::timeout")]
    pub timeout_secs: u64,

    /// Longer timeout for sitemap/discovery requests
    #[serde(default = "defaults::sitemap_timeout")]
    pub sitemap_timeout_secs: u64,

    /// Delay between requests in milliseconds
    #[serde(default = "defaults::request_delay")]
    pub request_delay_ms: u64,

    /// Maximum concurrent requests
    #[serde(default = "defaults::max_concurrent")]
    pub max_concurrent: usize,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            user_agent: defaults::user_agent(),
            timeout_secs: defaults::timeout(),
            sitemap_timeout_secs: defaults::sitemap_timeout(),
            request_delay_ms: defaults::request_delay(),
            max_concurrent: defaults::max_concurrent(),
        }
    }
}

/// Board discovery settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Maximum length for board name (longer text is likely a notice title)
    #[serde(default = "defaults::max_board_name_length")]
    pub max_board_name_length: usize,

    /// URL patterns to exclude from board discovery
    #[serde(default = "defaults::blacklist_patterns")]
    pub blacklist_patterns: Vec<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            max_board_name_length: defaults::max_board_name_length(),
            blacklist_patterns: defaults::blacklist_patterns(),
        }
    }
}

/// Text cleaning/preprocessing settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CleaningConfig {
    /// Patterns to remove from titles
    #[serde(default)]
    pub title_remove_patterns: Vec<String>,

    /// Patterns to remove from dates
    #[serde(default)]
    pub date_remove_patterns: Vec<String>,

    /// Text replacements to apply to dates
    #[serde(default)]
    pub date_replacements: Vec<Replacement>,
}

impl CleaningConfig {
    /// Clean text by removing patterns and applying replacements.
    fn clean(&self, text: &str, patterns: &[String], replacements: &[Replacement]) -> String {
        let mut result = Self::normalize_whitespace(text);

        for pattern in patterns {
            result = result.replace(pattern, "");
        }

        for r in replacements {
            result = result.replace(&r.from, &r.to);
        }

        result.trim().to_string()
    }

    /// Clean a title string.
    pub fn clean_title(&self, text: &str) -> String {
        self.clean(text, &self.title_remove_patterns, &[])
    }

    /// Clean a date string.
    pub fn clean_date(&self, text: &str) -> String {
        self.clean(text, &self.date_remove_patterns, &self.date_replacements)
    }

    fn normalize_whitespace(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// A text replacement rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}

/// Error message strings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct Errors {
    #[serde(default = "defaults::err_config_load")]
    pub config_load_failed: String,
    #[serde(default = "defaults::err_config_using_defaults")]
    pub config_using_defaults: String,
    #[serde(default = "defaults::err_seed_load_failed")]
    pub seed_load_failed: String,
    #[serde(default = "defaults::err_seed_using_defaults")]
    pub seed_using_defaults: String,
    #[serde(default = "defaults::err_seed_validation_failed")]
    pub seed_validation_failed: String,
    #[serde(default = "defaults::err_sitemap_not_found")]
    pub sitemap_not_found: String,
    #[serde(default = "defaults::err_invalid_date_format")]
    pub invalid_date_format: String,
    #[serde(default = "defaults::err_invalid_year")]
    pub invalid_year: String,
    #[serde(default = "defaults::err_invalid_month")]
    pub invalid_month: String,
    #[serde(default = "defaults::err_http_error")]
    pub http_error: String,
    #[serde(default = "defaults::err_parse_error")]
    pub parse_error: String,
}

mod defaults {
    use super::{CampusInfo, CmsPattern, KeywordMapping};

    // Crawler defaults
    pub fn user_agent() -> String {
        "Mozilla/5.0 (compatible; uRing/1.0)".into()
    }
    pub fn timeout() -> u64 {
        30
    }
    pub fn sitemap_timeout() -> u64 {
        10
    }
    pub fn request_delay() -> u64 {
        100
    }
    pub fn max_concurrent() -> usize {
        5
    }

    // Discovery defaults
    pub fn max_board_name_length() -> usize {
        20
    }
    pub fn blacklist_patterns() -> Vec<String> {
        vec![
            "articleNo".into(),
            "article_no".into(),
            "mode=view".into(),
            "seq".into(),
            "view.do".into(),
            "board_seq".into(),
        ]
    }

    // Campus defaults
    pub fn default_campuses() -> Vec<CampusInfo> {
        vec![
            CampusInfo {
                name: "신촌캠퍼스".to_string(),
                url: "https://www.yonsei.ac.kr/sc/186/subview.do".to_string(),
            },
            CampusInfo {
                name: "미래캠퍼스".to_string(),
                url: "https://mirae.yonsei.ac.kr/wj/1413/subview.do".to_string(),
            },
        ]
    }

    // Keyword defaults
    pub fn default_keywords() -> Vec<KeywordMapping> {
        vec![
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
            KeywordMapping {
                keyword: "공지".to_string(),
                id: "notice".to_string(),
                display_name: "일반공지".to_string(),
            },
            KeywordMapping {
                keyword: "진로".to_string(),
                id: "career".to_string(),
                display_name: "취업/진로".to_string(),
            },
            KeywordMapping {
                keyword: "채용".to_string(),
                id: "career".to_string(),
                display_name: "채용정보".to_string(),
            },
            KeywordMapping {
                keyword: "알림".to_string(),
                id: "notice".to_string(),
                display_name: "알림".to_string(),
            },
        ]
    }

    // CMS pattern defaults
    pub fn default_cms_patterns() -> Vec<CmsPattern> {
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
                name: "nx_cms".to_string(),
                detect_url_contains: None,
                detect_html_contains: Some("yon_board".to_string()),
                row_selector: "table.bl_list tr:has(td.td-subject)".to_string(),
                title_selector: "td.td-subject a".to_string(),
                date_selector: "td.td-date".to_string(),
                link_attr: "href".to_string(),
            },
            CmsPattern {
                name: "nx_cms_alt".to_string(),
                detect_url_contains: None,
                detect_html_contains: Some("NX CMS".to_string()),
                row_selector: "table.bl_list tr:has(td.td-subject)".to_string(),
                title_selector: "td.td-subject a".to_string(),
                date_selector: "td.td-date".to_string(),
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

    // Error defaults
    pub fn err_config_load() -> String {
        "Failed to load config from {path}: {error}".into()
    }
    pub fn err_config_using_defaults() -> String {
        "Using default configuration".into()
    }
    pub fn err_seed_load_failed() -> String {
        "Failed to load seed from {path}: {error}".into()
    }
    pub fn err_seed_using_defaults() -> String {
        "Using default seed data".into()
    }
    pub fn err_seed_validation_failed() -> String {
        "Seed validation failed: {error}".into()
    }
    pub fn err_sitemap_not_found() -> String {
        "Sitemap not found at {path}. Please run 'uRing map' first".into()
    }
    pub fn err_invalid_date_format() -> String {
        "Invalid date format. Use YYYY-MM (e.g., 2025-01)".into()
    }
    pub fn err_invalid_year() -> String {
        "Invalid year".into()
    }
    pub fn err_invalid_month() -> String {
        "Invalid month".into()
    }
    pub fn err_http_error() -> String {
        "HTTP error: {error}".into()
    }
    pub fn err_parse_error() -> String {
        "Parse error: {error}".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_default_config_ok() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_user_agent() {
        let mut config = Config::default();
        config.crawler.user_agent = "  ".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_zero_concurrency() {
        let mut config = Config::default();
        config.crawler.max_concurrent = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_accepts_valid_campuses_and_keywords() {
        let config = Config::default();
        assert!(config.validate().is_ok());
        assert!(!config.campuses.is_empty());
        assert!(!config.keywords.is_empty());
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
