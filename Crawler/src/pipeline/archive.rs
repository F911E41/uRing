// src/pipeline/archive.rs

use crate::error::Result;
use crate::models::{Config, LocaleConfig};
use crate::storage::{LocalStorage, NoticeStorage};
use crate::utils::log;

/// Archive new notices to monthly storage.
pub async fn run_archive(config: &Config, locale: &LocaleConfig) -> Result<()> {
    log::header(&locale.messages.archive_starting);

    let storage = LocalStorage::new(&config.paths.output);
    let metadata = storage.rotate_to_archive().await?;

    log::success(
        &locale
            .messages
            .archive_complete
            .replace("{count}", &metadata.notice_count.to_string()),
    );
    log::sub_item(
        &locale
            .messages
            .archive_location
            .replace("{path}", &metadata.location),
    );
    log::sub_item(
        &locale
            .messages
            .archive_timestamp
            .replace("{time}", &metadata.timestamp.to_string()),
    );

    Ok(())
}
