// src/pipeline/crawl.rs

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{Datelike, Utc};

use crate::error::{AppError, Result};
use crate::models::{Campus, Config, LocaleConfig, Notice};
use crate::services::NoticeCrawler;
use crate::storage::{
    LocalStorage, NoticeStorage,
    paths::{campus_prefix, event_key, events_prefix, pointer_key, snapshot_key},
};
use crate::utils::{log, save_notices};

/// Run the notice crawler.
pub async fn run_crawler(
    config: &Config,
    locale: &LocaleConfig,
    base_path: &PathBuf,
) -> Result<()> {
    log::header(&locale.messages.crawler_starting);

    let sitemap_path = config.departments_boards_path(base_path);
    if !sitemap_path.exists() {
        let error_msg = locale
            .errors
            .sitemap_not_found
            .replace("{path}", &format!("{:?}", sitemap_path));
        return Err(AppError::discovery(error_msg));
    }

    log::info(
        &locale
            .messages
            .crawler_loading_sitemap
            .replace("{path}", &format!("{:?}", sitemap_path)),
    );

    let campuses =
        Campus::load_all(&sitemap_path).map_err(|e| AppError::crawl("loading sitemap", e))?;
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
    let notices = crawler.fetch_all(&campuses).await?;

    print_notices(&notices, config, locale);
    save_notices(&notices, config, locale)?;

    let storage = LocalStorage::new(&config.paths.output);
    let mut notices_by_campus: HashMap<String, Vec<Notice>> = HashMap::new();
    for notice in &notices {
        notices_by_campus
            .entry(notice.campus.clone())
            .or_default()
            .push(notice.clone());
    }

    for (campus, campus_notices) in notices_by_campus {
        let campus_storage = storage.with_campus(&campus);
        let summary = campus_storage.store_events(&campus_notices).await?;
        let snapshot_metadata = campus_storage
            .write_snapshot(&summary.stored_notices)
            .await?;

        log::success(
            &locale
                .messages
                .storage_saved
                .replace("{count}", &summary.stored_notices.len().to_string())
                .replace("{path}", &snapshot_metadata.snapshot_location),
        );

        if config.logging.show_progress {
            log::sub_item(&format!(
                "Stored {} of {} notices ({} skipped)",
                summary.stored_notices.len(),
                summary.total_count(),
                summary.skipped_count
            ));
            log::sub_item(&format!(
                "Snapshot pointer: {}",
                snapshot_metadata.pointer_location
            ));
            log::sub_item(&format!(
                "Snapshot timestamp: {}",
                snapshot_metadata.timestamp
            ));
        }
    }

    if config.logging.show_progress {
        let bucket_prefix = "uRing";
        let example_campus = "CampusA";
        let campus_root = campus_prefix(bucket_prefix, example_campus);
        let now = Utc::now();
        log::info(&locale.messages.storage_paths_header);
        log::sub_item(&format!(
            "Event example ({}): {}",
            example_campus,
            event_key(&campus_root, now.year(), now.month(), "notice_id")
        ));
        log::sub_item(&format!(
            "Events prefix: {}",
            events_prefix(&campus_root, now.year(), now.month())
        ));
        log::sub_item(&format!(
            "Snapshot: {}",
            snapshot_key(&campus_root, now)
        ));
        log::sub_item(&format!(
            "Pointer: {}",
            pointer_key(&campus_root)
        ));
    }

    log::success(&locale.messages.crawler_complete);

    Ok(())
}

fn print_notices(notices: &[Notice], config: &Config, locale: &LocaleConfig) {
    if !config.output.console_enabled {
        return;
    }

    log::info(
        &locale
            .messages
            .total_notices
            .replace("{count}", &notices.len().to_string()),
    );
    log::separator();

    for notice in notices {
        log::sub_item(&notice.format(&config.output.notice_format));
    }
}
