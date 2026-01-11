// src/pipeline/validate.rs

use std::path::PathBuf;

use crate::config::load_all;
use crate::error::Result;
use crate::models::LocaleConfig;
use crate::utils::log;

/// Validate configuration and seed data using load_all.
pub fn run_validate(locale: &LocaleConfig, base_path: &PathBuf) -> Result<()> {
    log::header(&locale.messages.validate_starting);

    match load_all(base_path) {
        Ok((config, seed)) => {
            log::success(&locale.messages.validate_config_success);
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

            log::success(&locale.messages.validate_seed_success);
            log::sub_item(
                &locale
                    .messages
                    .validate_campuses
                    .replace("{count}", &seed.campuses.len().to_string()),
            );
            log::sub_item(
                &locale
                    .messages
                    .validate_keywords
                    .replace("{count}", &seed.keywords.len().to_string()),
            );
            log::sub_item(
                &locale
                    .messages
                    .validate_patterns
                    .replace("{count}", &seed.cms_patterns.len().to_string()),
            );
            Ok(())
        }
        Err(e) => {
            log::error(
                &locale
                    .messages
                    .validate_failed
                    .replace("{error}", &e.to_string()),
            );
            Err(e)
        }
    }
}
