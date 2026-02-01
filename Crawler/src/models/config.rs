//! Application configuration structures.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

/// Root application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP and crawling behavior settings
    #[serde(default)]
    pub crawler: CrawlerConfig,

    /// File path settings
    #[serde(default)]
    pub paths: PathsConfig,

    /// Board discovery rules
    #[serde(default)]
    pub discovery: DiscoveryConfig,

    /// Text preprocessing settings
    #[serde(default)]
    pub cleaning: CleaningConfig,

    /// Output format settings
    #[serde(default)]
    pub output: OutputConfig,

    /// Logging settings
    #[serde(default)]
    pub logging: LoggingConfig,
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
        if self.paths.output_dir.trim().is_empty() {
            return Err(AppError::validation("paths.output_dir is empty"));
        }
        if self.paths.departments_file.trim().is_empty() {
            return Err(AppError::validation("paths.departments_file is empty"));
        }
        if self.paths.departments_boards_file.trim().is_empty() {
            return Err(AppError::validation(
                "paths.departments_boards_file is empty",
            ));
        }
        if self.paths.manual_review_file.trim().is_empty() {
            return Err(AppError::validation("paths.manual_review_file is empty"));
        }
        if self.discovery.max_board_name_length == 0 {
            return Err(AppError::validation(
                "discovery.max_board_name_length must be > 0",
            ));
        }
        Ok(())
    }

    // Path helper methods

    /// Get the full path to the output directory.
    pub fn output_dir(&self, base: &Path) -> PathBuf {
        base.join(&self.paths.output_dir)
    }

    /// Get the full path to the seed file.
    pub fn seed_path(&self, base: &Path) -> PathBuf {
        base.join(&self.paths.seed_file)
    }

    /// Get the full path to departments file.
    pub fn departments_path(&self, base: &Path) -> PathBuf {
        self.output_dir(base).join(&self.paths.departments_file)
    }

    /// Get the full path to departments with boards file.
    pub fn departments_boards_path(&self, base: &Path) -> PathBuf {
        self.output_dir(base)
            .join(&self.paths.departments_boards_file)
    }

    /// Get the full path to manual review file.
    pub fn manual_review_path(&self, base: &Path) -> PathBuf {
        self.output_dir(base).join(&self.paths.manual_review_file)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            crawler: CrawlerConfig::default(),
            paths: PathsConfig::default(),
            discovery: DiscoveryConfig::default(),
            cleaning: CleaningConfig::default(),
            output: OutputConfig::default(),
            logging: LoggingConfig::default(),
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

/// File path configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Path to seed file (relative to project root)
    #[serde(default = "defaults::seed_file")]
    pub seed_file: String,

    /// Output directory path (relative to project root)
    #[serde(default = "defaults::output_dir")]
    pub output_dir: String,

    /// Output filename for crawled data
    #[serde(default = "defaults::output_dir")]
    pub output: String,

    /// Departments list filename
    #[serde(default = "defaults::departments_file")]
    pub departments_file: String,

    /// Departments with boards filename
    #[serde(default = "defaults::departments_boards_file")]
    pub departments_boards_file: String,

    /// Manual review items filename
    #[serde(default = "defaults::manual_review_file")]
    pub manual_review_file: String,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            seed_file: defaults::seed_file(),
            output_dir: defaults::output_dir(),
            output: defaults::output_dir(),
            departments_file: defaults::departments_file(),
            departments_boards_file: defaults::departments_boards_file(),
            manual_review_file: defaults::manual_review_file(),
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

/// Output format settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Enable console output
    #[serde(default)]
    pub console_enabled: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            console_enabled: false,
        }
    }
}

/// Logging settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (debug, info, warn, error)
    #[serde(default = "defaults::log_level")]
    pub level: String,

    /// Show progress indicators
    #[serde(default = "defaults::show_progress")]
    pub show_progress: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: defaults::log_level(),
            show_progress: defaults::show_progress(),
        }
    }
}

/// Internationalization/localization settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleConfig {
    /// UI messages
    pub messages: Messages,

    /// Error messages
    #[serde(default)]
    pub errors: Errors,
}

impl LocaleConfig {
    /// Load locale from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Load locale or return default if loading fails.
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        Self::load(&path).unwrap_or_else(|e| {
            log::warn!(
                "Locale load failed from {:?}: {}. Using defaults.",
                path.as_ref(),
                e
            );
            Self::default()
        })
    }
}

impl Default for LocaleConfig {
    fn default() -> Self {
        Self {
            messages: Messages::default(),
            errors: Errors::default(),
        }
    }
}

/// UI message strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Messages {
    // Startup messages
    #[serde(default = "defaults::msg_app_starting")]
    pub app_starting: String,
    #[serde(default = "defaults::msg_app_finished")]
    pub app_finished: String,

    // Mapper messages
    #[serde(default = "defaults::msg_mapper_starting")]
    pub mapper_starting: String,
    #[serde(default = "defaults::msg_mapper_loading_seed")]
    pub mapper_loading_seed: String,
    #[serde(default = "defaults::msg_mapper_loaded_campuses")]
    pub mapper_loaded_campuses: String,
    #[serde(default = "defaults::msg_mapper_validated")]
    pub mapper_validated: String,
    #[serde(default = "defaults::msg_mapper_step_departments")]
    pub mapper_step_departments: String,
    #[serde(default = "defaults::msg_mapper_step_boards")]
    pub mapper_step_boards: String,
    #[serde(default = "defaults::msg_mapper_campus")]
    pub mapper_campus: String,
    #[serde(default = "defaults::msg_mapper_dept_scanning")]
    pub mapper_dept_scanning: String,
    #[serde(default = "defaults::msg_mapper_dept_accessed")]
    pub mapper_dept_accessed: String,
    #[serde(default = "defaults::msg_mapper_dept_found_boards")]
    pub mapper_dept_found_boards: String,
    #[serde(default = "defaults::msg_mapper_sitemap_found")]
    pub mapper_sitemap_found: String,
    #[serde(default = "defaults::msg_mapper_sitemap_fallback")]
    pub mapper_sitemap_fallback: String,
    #[serde(default = "defaults::msg_mapper_complete")]
    pub mapper_complete: String,
    #[serde(default = "defaults::msg_mapper_manual_review")]
    pub mapper_manual_review: String,

    // Crawler messages
    #[serde(default = "defaults::msg_starting")]
    pub crawler_starting: String,
    #[serde(default = "defaults::msg_crawler_loading_sitemap")]
    pub crawler_loading_sitemap: String,
    #[serde(default = "defaults::msg_loaded")]
    pub loaded_departments: String,
    #[serde(default = "defaults::msg_crawler_fetching")]
    pub crawler_fetching: String,
    #[serde(default = "defaults::msg_crawler_fetch_error")]
    pub crawler_fetch_error: String,
    #[serde(default = "defaults::msg_crawler_complete")]
    pub crawler_complete: String,
    #[serde(default = "defaults::msg_total")]
    pub total_notices: String,
    #[serde(default = "defaults::msg_saved")]
    pub saved_notices: String,
    #[serde(default = "defaults::msg_storage_saved")]
    pub storage_saved: String,
    #[serde(default = "defaults::msg_storage_paths_header")]
    pub storage_paths_header: String,

    // Archive messages
    #[serde(default = "defaults::msg_archive_starting")]
    pub archive_starting: String,
    #[serde(default = "defaults::msg_archive_complete")]
    pub archive_complete: String,
    #[serde(default = "defaults::msg_archive_location")]
    pub archive_location: String,
    #[serde(default = "defaults::msg_archive_timestamp")]
    pub archive_timestamp: String,

    // Load messages
    #[serde(default = "defaults::msg_load_new")]
    pub load_new: String,
    #[serde(default = "defaults::msg_load_archive")]
    pub load_archive: String,
    #[serde(default = "defaults::msg_load_complete")]
    pub load_complete: String,
    #[serde(default = "defaults::msg_load_notice_item")]
    pub load_notice_item: String,

    // Validate messages
    #[serde(default = "defaults::msg_validate_starting")]
    pub validate_starting: String,
    #[serde(default = "defaults::msg_validate_config_success")]
    pub validate_config_success: String,
    #[serde(default = "defaults::msg_validate_seed_success")]
    pub validate_seed_success: String,
    #[serde(default = "defaults::msg_validate_user_agent")]
    pub validate_user_agent: String,
    #[serde(default = "defaults::msg_validate_timeout")]
    pub validate_timeout: String,
    #[serde(default = "defaults::msg_validate_max_concurrent")]
    pub validate_max_concurrent: String,
    #[serde(default = "defaults::msg_validate_campuses")]
    pub validate_campuses: String,
    #[serde(default = "defaults::msg_validate_keywords")]
    pub validate_keywords: String,
    #[serde(default = "defaults::msg_validate_patterns")]
    pub validate_patterns: String,
    #[serde(default = "defaults::msg_validate_failed")]
    pub validate_failed: String,

    // Pipeline messages
    #[serde(default = "defaults::msg_pipeline_starting")]
    pub pipeline_starting: String,
    #[serde(default = "defaults::msg_pipeline_step")]
    pub pipeline_step: String,
    #[serde(default = "defaults::msg_pipeline_complete")]
    pub pipeline_complete: String,

    // Department crawler messages
    #[serde(default = "defaults::msg_dept_crawling")]
    pub dept_crawling: String,
    #[serde(default = "defaults::msg_dept_found")]
    pub dept_found: String,
    #[serde(default = "defaults::msg_dept_failed")]
    pub dept_failed: String,
    #[serde(default = "defaults::msg_dept_no_content")]
    pub dept_no_content: String,
    #[serde(default = "defaults::msg_dept_no_homepage")]
    pub dept_no_homepage: String,

    // CMS detection messages
    #[serde(default = "defaults::msg_cms_detected")]
    pub cms_detected: String,

    // Summary labels
    #[serde(default = "defaults::msg_summary_total_depts")]
    pub summary_total_depts: String,
    #[serde(default = "defaults::msg_summary_total_boards")]
    pub summary_total_boards: String,
    #[serde(default = "defaults::msg_summary_manual_review")]
    pub summary_manual_review: String,
    #[serde(default = "defaults::msg_summary_notices")]
    pub summary_notices: String,

    // Separators
    #[serde(default = "defaults::msg_separator")]
    pub separator_line: String,
    #[serde(default = "defaults::msg_separator_short")]
    pub separator_short: String,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            app_starting: defaults::msg_app_starting(),
            app_finished: defaults::msg_app_finished(),
            mapper_starting: defaults::msg_mapper_starting(),
            mapper_loading_seed: defaults::msg_mapper_loading_seed(),
            mapper_loaded_campuses: defaults::msg_mapper_loaded_campuses(),
            mapper_validated: defaults::msg_mapper_validated(),
            mapper_step_departments: defaults::msg_mapper_step_departments(),
            mapper_step_boards: defaults::msg_mapper_step_boards(),
            mapper_campus: defaults::msg_mapper_campus(),
            mapper_dept_scanning: defaults::msg_mapper_dept_scanning(),
            mapper_dept_accessed: defaults::msg_mapper_dept_accessed(),
            mapper_dept_found_boards: defaults::msg_mapper_dept_found_boards(),
            mapper_sitemap_found: defaults::msg_mapper_sitemap_found(),
            mapper_sitemap_fallback: defaults::msg_mapper_sitemap_fallback(),
            mapper_complete: defaults::msg_mapper_complete(),
            mapper_manual_review: defaults::msg_mapper_manual_review(),
            crawler_starting: defaults::msg_starting(),
            crawler_loading_sitemap: defaults::msg_crawler_loading_sitemap(),
            loaded_departments: defaults::msg_loaded(),
            crawler_fetching: defaults::msg_crawler_fetching(),
            crawler_fetch_error: defaults::msg_crawler_fetch_error(),
            crawler_complete: defaults::msg_crawler_complete(),
            total_notices: defaults::msg_total(),
            saved_notices: defaults::msg_saved(),
            storage_saved: defaults::msg_storage_saved(),
            storage_paths_header: defaults::msg_storage_paths_header(),
            archive_starting: defaults::msg_archive_starting(),
            archive_complete: defaults::msg_archive_complete(),
            archive_location: defaults::msg_archive_location(),
            archive_timestamp: defaults::msg_archive_timestamp(),
            load_new: defaults::msg_load_new(),
            load_archive: defaults::msg_load_archive(),
            load_complete: defaults::msg_load_complete(),
            load_notice_item: defaults::msg_load_notice_item(),
            validate_starting: defaults::msg_validate_starting(),
            validate_config_success: defaults::msg_validate_config_success(),
            validate_seed_success: defaults::msg_validate_seed_success(),
            validate_user_agent: defaults::msg_validate_user_agent(),
            validate_timeout: defaults::msg_validate_timeout(),
            validate_max_concurrent: defaults::msg_validate_max_concurrent(),
            validate_campuses: defaults::msg_validate_campuses(),
            validate_keywords: defaults::msg_validate_keywords(),
            validate_patterns: defaults::msg_validate_patterns(),
            validate_failed: defaults::msg_validate_failed(),
            pipeline_starting: defaults::msg_pipeline_starting(),
            pipeline_step: defaults::msg_pipeline_step(),
            pipeline_complete: defaults::msg_pipeline_complete(),
            dept_crawling: defaults::msg_dept_crawling(),
            dept_found: defaults::msg_dept_found(),
            dept_failed: defaults::msg_dept_failed(),
            dept_no_content: defaults::msg_dept_no_content(),
            dept_no_homepage: defaults::msg_dept_no_homepage(),
            cms_detected: defaults::msg_cms_detected(),
            summary_total_depts: defaults::msg_summary_total_depts(),
            summary_total_boards: defaults::msg_summary_total_boards(),
            summary_manual_review: defaults::msg_summary_manual_review(),
            summary_notices: defaults::msg_summary_notices(),
            separator_line: defaults::msg_separator(),
            separator_short: defaults::msg_separator_short(),
        }
    }
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

    // Path defaults
    pub fn seed_file() -> String {
        "data/seed.toml".into()
    }
    pub fn output_dir() -> String {
        "data/output".into()
    }
    pub fn manual_review_file() -> String {
        "Temp/manual_review_needed.json".into()
    }
    pub fn departments_file() -> String {
        "Temp/yonsei_departments.json".into()
    }
    pub fn departments_boards_file() -> String {
        "siteMap.json".into()
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

    // Output defaults

    // Logging defaults
    pub fn log_level() -> String {
        "info".into()
    }
    pub fn show_progress() -> bool {
        true
    }

    // Startup message defaults
    pub fn msg_app_starting() -> String {
        "uRing Crawler v1.0.0 starting".into()
    }
    pub fn msg_app_finished() -> String {
        "uRing Crawler finished".into()
    }

    // Mapper message defaults
    pub fn msg_mapper_starting() -> String {
        "Starting Mapper Mode".into()
    }
    pub fn msg_mapper_loading_seed() -> String {
        "Loading seed data from {path}".into()
    }
    pub fn msg_mapper_loaded_campuses() -> String {
        "Loaded {count} campuses".into()
    }
    pub fn msg_mapper_validated() -> String {
        "Seed data validated successfully".into()
    }
    pub fn msg_mapper_step_departments() -> String {
        "Crawling departments".into()
    }
    pub fn msg_mapper_step_boards() -> String {
        "Discovering boards".into()
    }
    pub fn msg_mapper_campus() -> String {
        "Processing campus: {name}".into()
    }
    pub fn msg_mapper_dept_scanning() -> String {
        "Scanning {name}".into()
    }
    pub fn msg_mapper_dept_accessed() -> String {
        "Accessed {url}".into()
    }
    pub fn msg_mapper_dept_found_boards() -> String {
        "Found {count} board(s)".into()
    }
    pub fn msg_mapper_sitemap_found() -> String {
        "Found sitemap: {url}".into()
    }
    pub fn msg_mapper_sitemap_fallback() -> String {
        "Sitemap yielded no results, falling back to homepage".into()
    }
    pub fn msg_mapper_complete() -> String {
        "Mapper complete! Data saved to {path}".into()
    }
    pub fn msg_mapper_manual_review() -> String {
        "Saved {count} items needing manual review to {path}".into()
    }

    // Crawler message defaults
    pub fn msg_starting() -> String {
        "Starting Crawler Mode".into()
    }
    pub fn msg_crawler_loading_sitemap() -> String {
        "Loading sitemap from {path}".into()
    }
    pub fn msg_loaded() -> String {
        "Loaded {count_dept} department(s) with {count_board} board(s)".into()
    }
    pub fn msg_crawler_fetching() -> String {
        "Fetching notices from boards".into()
    }
    pub fn msg_crawler_fetch_error() -> String {
        "Error fetching {dept}/{board}: {error}".into()
    }
    pub fn msg_crawler_complete() -> String {
        "Crawl complete".into()
    }
    pub fn msg_total() -> String {
        "Total notices fetched: {count}".into()
    }
    pub fn msg_saved() -> String {
        "Saved notices to {path}".into()
    }
    pub fn msg_storage_saved() -> String {
        "Storage: {count} notices saved to {path}".into()
    }
    pub fn msg_storage_paths_header() -> String {
        "S3 Storage Paths (events + snapshots)".into()
    }

    // Archive message defaults
    pub fn msg_archive_starting() -> String {
        "Archiving notices".into()
    }
    pub fn msg_archive_complete() -> String {
        "Archived {count} notices".into()
    }
    pub fn msg_archive_location() -> String {
        "Location: {path}".into()
    }
    pub fn msg_archive_timestamp() -> String {
        "Timestamp: {time}".into()
    }

    // Load message defaults
    pub fn msg_load_new() -> String {
        "Loading latest snapshot".into()
    }
    pub fn msg_load_archive() -> String {
        "Loading event notices from {year}-{month}".into()
    }
    pub fn msg_load_complete() -> String {
        "Loaded {count} notices".into()
    }
    pub fn msg_load_notice_item() -> String {
        "{title} [{date}]".into()
    }

    // Validate message defaults
    pub fn msg_validate_starting() -> String {
        "Validating configuration and seed data".into()
    }
    pub fn msg_validate_config_success() -> String {
        "Configuration loaded successfully".into()
    }
    pub fn msg_validate_seed_success() -> String {
        "Seed data validated".into()
    }
    pub fn msg_validate_user_agent() -> String {
        "User agent: {value}".into()
    }
    pub fn msg_validate_timeout() -> String {
        "Timeout: {value}s".into()
    }
    pub fn msg_validate_max_concurrent() -> String {
        "Max concurrent: {value}".into()
    }
    pub fn msg_validate_campuses() -> String {
        "Campuses: {count}".into()
    }
    pub fn msg_validate_keywords() -> String {
        "Keywords: {count}".into()
    }
    pub fn msg_validate_patterns() -> String {
        "CMS patterns: {count}".into()
    }
    pub fn msg_validate_failed() -> String {
        "Validation failed: {error}".into()
    }

    // Pipeline message defaults
    pub fn msg_pipeline_starting() -> String {
        "Starting Pipeline Mode".into()
    }
    pub fn msg_pipeline_step() -> String {
        "Pipeline step {current}/{total}: {name}".into()
    }
    pub fn msg_pipeline_complete() -> String {
        "Pipeline complete".into()
    }

    // Department crawler message defaults
    pub fn msg_dept_crawling() -> String {
        "Crawling {name}".into()
    }
    pub fn msg_dept_found() -> String {
        "Found {count} departments".into()
    }
    pub fn msg_dept_failed() -> String {
        "Failed to crawl {name}: {error}".into()
    }
    pub fn msg_dept_no_content() -> String {
        "Cannot find main content area for {name}".into()
    }
    pub fn msg_dept_no_homepage() -> String {
        "No homepage URL found for {name}".into()
    }

    // CMS detection message defaults
    pub fn msg_cms_detected() -> String {
        "Detected CMS pattern: {name} for URL: {url}".into()
    }

    // Summary label defaults
    pub fn msg_summary_total_depts() -> String {
        "Total Departments".into()
    }
    pub fn msg_summary_total_boards() -> String {
        "Total Boards Found".into()
    }
    pub fn msg_summary_manual_review() -> String {
        "Needs Manual Review".into()
    }
    pub fn msg_summary_notices() -> String {
        "Notices Fetched".into()
    }

    // Separator defaults
    pub fn msg_separator() -> String {
        "═".into()
    }
    pub fn msg_separator_short() -> String {
        "─".into()
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
}
