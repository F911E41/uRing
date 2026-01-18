// src/pipeline/validate.rs

use std::path::PathBuf;

use crate::config::load_all;
use crate::error::Result;
use crate::models::LocaleConfig;
use crate::utils::log;

/// Validate configuration and seed data.
/// Checks for both syntax errors (parsing) and logical issues (empty lists, invalid values).
pub fn run_validate(locale: &LocaleConfig, base_path: &PathBuf) -> Result<()> {
    log::header(&locale.messages.validate_starting);

    // Load config and seed files
    let (config, seed) = load_all(base_path).map_err(|e| {
        log::error(
            &locale
                .messages
                .validate_failed
                .replace("{error}", &e.to_string()),
        );
        e
    })?;

    // Validate config and log results
    log::success(&locale.messages.validate_config_success);

    // Warning for potentially problematic config values
    if config.crawler.timeout_secs == 0 {
        log::warn("Crawler timeout is set to 0 seconds. Requests might fail instantly.");
    }

    log::sub_item(
        &locale
            .messages
            .validate_user_agent
            .replace("{value}", &config.crawler.user_agent),
    );
    log::sub_item(
        &locale
            .messages
            .validate_timeout
            .replace("{value}", &config.crawler.timeout_secs.to_string()),
    );
    log::sub_item(
        &locale
            .messages
            .validate_max_concurrent
            .replace("{value}", &config.crawler.max_concurrent.to_string()),
    );

    // Seed validation and logging
    log::success(&locale.messages.validate_seed_success);

    // Check for empty lists and log warnings
    if seed.campuses.is_empty() {
        log::error("No campuses found in seed data. The crawler will have nothing to do.");
        // If desired, could return an error here to halt execution
        // return Err(Error::Config("Seed data contains no campuses".into()));
    } else {
        log::sub_item(
            &locale
                .messages
                .validate_campuses
                .replace("{count}", &seed.campuses.len().to_string()),
        );
    }

    if seed.keywords.is_empty() {
        log::warn("No keywords defined. Filtering might be ineffective.");
    }

    log::sub_item(
        &locale
            .messages
            .validate_keywords
            .replace("{count}", &seed.keywords.len().to_string()),
    );

    if seed.cms_patterns.is_empty() {
        log::warn(
            "No CMS patterns defined. Board detection might rely solely on fallback methods.",
        );
    }

    log::sub_item(
        &locale
            .messages
            .validate_patterns
            .replace("{count}", &seed.cms_patterns.len().to_string()),
    );

    Ok(())
}
