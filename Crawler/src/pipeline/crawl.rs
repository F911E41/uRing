//! Notice crawling pipeline.
//!
//! Fetches notices from discovered boards and saves using Hot/Cold pattern.

use std::sync::Arc;

use chrono::Utc;
use reqwest::Client;

use crate::error::Result;
use crate::models::{Campus, Config, CrawlStats, LocaleConfig};
use crate::services::NoticeCrawler;
use crate::storage::NoticeStorage;

/// Run the notice crawler.
pub async fn run_crawler(
    config: Arc<Config>,
    locale: &LocaleConfig,
    storage: &impl NoticeStorage,
    campuses: &[Campus],
    client: &Client,
) -> Result<()> {
    let start_time = Utc::now();
    log::info!("{}", locale.messages.crawler_starting);

    let total_depts: usize = campuses.iter().map(|c| c.department_count()).sum();
    let total_boards: usize = campuses.iter().map(|c| c.board_count()).sum();

    log::info!(
        "Loaded {} departments with {} boards",
        total_depts,
        total_boards
    );

    log::info!("{}", locale.messages.crawler_fetching);

    // Initialize the crawler with Config and Client
    let crawler = NoticeCrawler::new(Arc::clone(&config), client.clone())?;

    // Run the crawler to fetch all notices
    let outcome = crawler.fetch_all(campuses).await?;
    let end_time = Utc::now();

    // Calculate success rates
    let calc_rate = |total: usize, fail: usize| -> f32 {
        if total == 0 {
            0.0
        } else {
            (total - fail) as f32 / total as f32
        }
    };

    let stats = CrawlStats {
        start_time,
        end_time,
        notice_count: outcome.notices.len(),
        department_count: total_depts,
        board_count: total_boards,
        board_total: outcome.board_total,
        board_failures: outcome.board_failures,
        board_success_rate: calc_rate(outcome.board_total, outcome.board_failures),
        notice_total: outcome.notice_total,
        notice_failures: outcome.notice_failures,
        notice_success_rate: calc_rate(outcome.notice_total, outcome.notice_failures),
        detail_total: outcome.detail_total,
        detail_failures: outcome.detail_failures,
        detail_success_rate: calc_rate(outcome.detail_total, outcome.detail_failures),
    };

    // Write using Hot/Cold storage pattern
    let metadata = storage.write_notices(&outcome, campuses, &stats).await?;

    log::info!(
        "Saved {} hot notices + {} cold archive files",
        metadata.hot_count,
        metadata.cold_files_updated
    );

    log::info!("{}", locale.messages.crawler_complete);

    if outcome.board_failures > 0 || outcome.notice_failures > 0 || outcome.detail_failures > 0 {
        log::warn!(
            "Crawl completed with issues: {} board fails, {} notice fails, {} detail fails",
            outcome.board_failures,
            outcome.notice_failures,
            outcome.detail_failures
        );
    }

    Ok(())
}
