// src/models/config.rs

use serde::Deserialize;

/// Root configuration structure
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub crawler: CrawlerConfig,
    pub paths: PathsConfig,
    pub cleaning: CleaningConfig,
    pub output: OutputConfig,
    pub logging: LoggingConfig,
}

/// Locale configuration structure
#[derive(Debug, Deserialize, Clone)]
pub struct LocaleConfig {
    #[allow(dead_code)] // Reserved for future CLI localization
    pub cli: CliLocale,
    pub messages: MessageLocale,
    pub errors: ErrorLocale,
}

/// CLI text localization
#[derive(Debug, Deserialize, Clone)]
pub struct CliLocale {
    #[allow(dead_code)] // Reserved for future CLI description localization
    pub description: String,
    #[allow(dead_code)] // Reserved for future CLI help text localization
    pub config_help: String,
    #[allow(dead_code)] // Reserved for future CLI help text localization
    pub site_map_help: String,
    #[allow(dead_code)] // Reserved for future CLI help text localization
    pub output_help: String,
    #[allow(dead_code)] // Reserved for future CLI help text localization
    pub quiet_help: String,
}

/// Message text localization
#[derive(Debug, Deserialize, Clone)]
pub struct MessageLocale {
    pub crawler_starting: String,
    pub loaded_departments: String,
    pub department_header: String,
    pub board_success: String,
    pub board_error: String,
    pub total_notices: String,
    pub saved_notices: String,
    pub separator_line: String,
    pub separator_short: String,
}

/// Error text localization
#[derive(Debug, Deserialize, Clone)]
pub struct ErrorLocale {
    pub config_load_failed: String,
}

/// Crawler behavior settings
#[derive(Debug, Deserialize, Clone)]
pub struct CrawlerConfig {
    pub user_agent: String,
    pub timeout_secs: u64,
    pub request_delay_ms: u64,
    #[allow(dead_code)] // Reserved for future concurrent crawling
    pub max_concurrent: usize,
}

/// File path configurations
#[derive(Debug, Deserialize, Clone)]
pub struct PathsConfig {
    pub site_map: String,
    pub output: String,
}

/// Text cleaning configurations
#[derive(Debug, Deserialize, Clone)]
pub struct CleaningConfig {
    pub title_remove_patterns: Vec<String>,
    pub date_remove_patterns: Vec<String>,
    #[serde(default)]
    pub date_replacements: Vec<Replacement>,
}

/// A text replacement rule
#[derive(Debug, Deserialize, Clone)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}

/// Output format configurations
#[derive(Debug, Deserialize, Clone)]
pub struct OutputConfig {
    pub console_enabled: bool,
    pub json_enabled: bool,
    pub json_pretty: bool,
    pub notice_format: String,
}

/// Logging configurations
#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    #[allow(dead_code)] // Reserved for future log filtering
    pub level: String,
    pub show_progress: bool,
}
