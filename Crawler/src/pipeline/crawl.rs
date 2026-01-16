// src/pipeline/crawl.rs

//! Notice crawling pipeline.

use std::sync::Arc;

use chrono::Utc;

use crate::error::Result;
use crate::models::{Campus, Config, CrawlStats, LocaleConfig};
use crate::services::NoticeCrawler;
use crate::storage::NoticeStorage;
use crate::utils::log;

/// Run the notice crawler.
pub async fn run_crawler(
    config: &Config,
    locale: &LocaleConfig,
    storage: &dyn NoticeStorage,
    campuses: &[Campus],
) -> Result<()> {
    let start_time = Utc::now();
    log::header(&locale.messages.crawler_starting);

    let total_depts: usize = campuses.iter().map(|c| c.department_count()).sum();
    let total_boards: usize = campuses.iter().map(|c| c.board_count()).sum();

    log::info(
        &locale
            .messages
            .loaded_departments
            .replace("{count_dept}", &total_depts.to_string())
            .replace("{count_board}", &total_boards.to_string()),
    );

    log::info(&locale.messages.crawler_fetching);

    let crawler = NoticeCrawler::new(Arc::new(config.clone()));
    let notices = crawler.fetch_all(campuses).await?;

    let end_time = Utc::now();
    let stats = CrawlStats {
        start_time,
        end_time,
        notice_count: notices.len(),
        department_count: total_depts,
        board_count: total_boards,
        success_rate: 1.0, // Placeholder
    };

    let summary = storage.write_snapshot(&notices, campuses, &stats).await?;

    log::success(
        &locale
            .messages
            .storage_saved
            .replace("{count}", &summary.notice_count.to_string())
            .replace("{path}", &summary.snapshot_location),
    );

    if config.logging.show_progress {
        log::sub_item(&format!("Snapshot pointer: {}", summary.pointer_location));
        log::sub_item(&format!("Snapshot timestamp: {}", summary.timestamp));
    }

    log::success(&locale.messages.crawler_complete);

    Ok(())
}
