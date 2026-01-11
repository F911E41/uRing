// src/pipeline/load.rs

use crate::error::{AppError, Result};
use crate::models::{Config, LocaleConfig};
use crate::storage::{LocalStorage, NoticeStorage};
use crate::utils::log;

/// Load notices from storage.
pub async fn run_load(config: &Config, locale: &LocaleConfig, from: &str) -> Result<()> {
    let storage = LocalStorage::new(&config.paths.output);

    let notices = if from == "new" {
        log::info(&locale.messages.load_new);
        storage.load_new().await?
    } else {
        let parts: Vec<&str> = from.split('-').collect();
        if parts.len() != 2 {
            return Err(AppError::validation(&locale.errors.invalid_date_format));
        }
        let year: i32 = parts[0]
            .parse()
            .map_err(|_| AppError::validation(&locale.errors.invalid_year))?;
        let month: u32 = parts[1]
            .parse()
            .map_err(|_| AppError::validation(&locale.errors.invalid_month))?;

        log::info(
            &locale
                .messages
                .load_archive
                .replace("{year}", &year.to_string())
                .replace("{month}", &format!("{:02}", month)),
        );
        storage.load_archive(year, month).await?
    };

    log::success(
        &locale
            .messages
            .load_complete
            .replace("{count}", &notices.len().to_string()),
    );
    for notice in &notices {
        log::sub_item(
            &locale
                .messages
                .load_notice_item
                .replace("{title}", &notice.title)
                .replace("{date}", &notice.date),
        );
    }

    Ok(())
}
