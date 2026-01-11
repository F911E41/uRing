// src/pipeline/crawl.rs

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{Datelike, Utc};

use crate::error::{AppError, Result};
use crate::models::{Campus, Config, LocaleConfig, Notice};
use crate::services::NoticeCrawler;
use crate::storage::{
    LocalStorage, NoticeStorage,
    paths::{campus_prefix, monthly_archive_key, monthly_prefix, new_notices_key},
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
    let metadata = storage.store_new(&notices).await?;

    log::success(
        &locale
            .messages
            .storage_saved
            .replace("{count}", &metadata.notice_count.to_string())
            .replace("{path}", &metadata.location),
    );

    if config.logging.show_progress {
        let bucket_prefix = "uRing";
        let example_campus = "CampusA";
        let campus_root = campus_prefix(bucket_prefix, example_campus);
        let now = Utc::now();
        log::info(&locale.messages.storage_paths_header);
        log::sub_item(&format!(
            "New notices ({}): {}",
            example_campus,
            new_notices_key(&campus_root)
        ));
        log::sub_item(&format!(
            "Archive: {}",
            monthly_archive_key(&campus_root, now)
        ));
        log::sub_item(&format!(
            "Monthly prefix: {}",
            monthly_prefix(&campus_root, now.year(), now.month())
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
