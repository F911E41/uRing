// src/config.rs

use std::error::Error;
use std::fs;
use std::path::Path;

use crate::models::config::{
    CleaningConfig, Config, CrawlerConfig, LocaleConfig, LoggingConfig, OutputConfig, PathsConfig,
    Replacement,
};

impl Config {
    /// Load configuration from a TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration with fallback to defaults
    pub fn load_or_default<P: AsRef<Path>>(path: P, locale: &LocaleConfig) -> Self {
        match Self::load(path) {
            Ok(config) => config,
            Err(e) => {
                eprintln!(
                    "{}",
                    locale
                        .errors
                        .config_load_failed
                        .replace("{}", &format!("{}", e))
                );
                Self::default()
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            crawler: CrawlerConfig {
                user_agent: "Mozilla/5.0 (compatible; uRing Crawler/0.1)".to_string(),
                timeout_secs: 30,
                request_delay_ms: 100,
                max_concurrent: 0,
            },
            paths: PathsConfig {
                site_map: "data/siteMap.json".to_string(),
                output: "data/result/notices.json".to_string(),
            },
            cleaning: CleaningConfig {
                title_remove_patterns: vec!["첨부파일".to_string(), "공지 ".to_string()],
                date_remove_patterns: vec!["작성일".to_string()],
                date_replacements: vec![Replacement {
                    from: ". ".to_string(),
                    to: ".".to_string(),
                }],
            },
            output: OutputConfig {
                console_enabled: false,
                json_enabled: true,
                json_pretty: true,
                notice_format: "📌 [{dept_name}:{board_name}] {title}\n   📅 {date}\n   🔗 {link}"
                    .to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                show_progress: true,
            },
        }
    }
}

/// Apply cleaning patterns to title text
pub fn clean_title(s: &str, config: &CleaningConfig) -> String {
    let mut result = normalize_whitespace(s);
    for pattern in &config.title_remove_patterns {
        result = result.replace(pattern, "");
    }
    result.trim().to_string()
}

/// Apply cleaning patterns to date text
pub fn clean_date(s: &str, config: &CleaningConfig) -> String {
    let mut result = normalize_whitespace(s);
    for pattern in &config.date_remove_patterns {
        result = result.replace(pattern, "");
    }
    for replacement in &config.date_replacements {
        result = result.replace(&replacement.from, &replacement.to);
    }
    result.trim().to_string()
}

/// Normalize whitespace: collapse multiple spaces/newlines into single space
pub fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Format a notice for console output
pub fn format_notice(
    format: &str,
    dept_name: &str,
    board_name: &str,
    title: &str,
    date: &str,
    link: &str,
) -> String {
    format
        .replace("{dept_name}", dept_name)
        .replace("{board_name}", board_name)
        .replace("{title}", title)
        .replace("{date}", date)
        .replace("{link}", link)
}
