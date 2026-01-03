// src/locale.rs

use std::error::Error;
use std::fs;
use std::path::Path;

use crate::models::config::LocaleConfig;

/// Load locale configuration from a TOML file
pub fn load_locale<P: AsRef<Path>>(path: P) -> Result<LocaleConfig, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let locale: LocaleConfig = toml::from_str(&content)?;
    Ok(locale)
}

/// Load locale configuration with fallback to defaults
pub fn load_locale_or_default<P: AsRef<Path>>(path: P) -> LocaleConfig {
    match load_locale(path) {
        Ok(locale) => locale,
        Err(e) => {
            eprintln!("⚠️  Failed to load locale: {}. Using defaults.", e);
            LocaleConfig::default()
        }
    }
}

impl Default for LocaleConfig {
    fn default() -> Self {
        LocaleConfig {
            cli: CliLocale {
                description: "Fetch notices from university department websites".to_string(),
                config_help: "Path to configuration file".to_string(),
                site_map_help: "Override site map path".to_string(),
                output_help: "Override output path".to_string(),
                quiet_help: "Suppress console output".to_string(),
            },
            messages: MessageLocale {
                crawler_starting: "🕷️  uRing Crawler starting...\n".to_string(),
                loaded_departments: "📋 Loaded {} department(s) with {} board(s)\n".to_string(),
                department_header: "📂 {}".to_string(),
                board_success: "   ✓ {} - {} notices".to_string(),
                board_error: "   ✗ {} - Error: {}".to_string(),
                total_notices: "\n📰 Total notices fetched: {}\n".to_string(),
                saved_notices: "\n💾 Saved notices to {}".to_string(),
                separator_line: "=".to_string(),
                separator_short: "-".to_string(),
            },
            errors: ErrorLocale {
                config_load_failed: "⚠️  Failed to load config: {}. Using defaults.".to_string(),
            },
        }
    }
}

use crate::models::config::{CliLocale, ErrorLocale, MessageLocale};
