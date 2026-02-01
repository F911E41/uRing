//! Mapper pipeline.
//!
//! Department and board discovery pipeline.

use std::sync::Arc;

use futures::{StreamExt, stream};
use reqwest::Client;

use crate::error::Result;
use crate::models::{Campus, Config, LocaleConfig, ManualReviewItem, Seed};
use crate::services::{BoardDiscoveryService, DepartmentCrawler, SelectorDetector};

/// Maximum concurrency for board discovery.
const CONCURRENCY_LIMIT: usize = 14;

/// Run the mapper to discover departments and boards.
pub async fn run_mapper(
    config: &Config,
    locale: &LocaleConfig,
    seed: &Seed,
    client: &Client,
) -> Result<Vec<Campus>> {
    log::info!("{}", locale.messages.mapper_starting);

    seed.validate()?;
    log::info!("Loaded {} campuses from seed", seed.campuses.len());

    // Step 1: Departments Discovery
    log::info!("[1/2] {}", locale.messages.mapper_step_departments);

    let dept_crawler = DepartmentCrawler::new(client);
    let mut campuses = dept_crawler.crawl_all(&seed.campuses).await?;

    if campuses.is_empty() {
        log::error!("No campuses discovered");
        return Ok(Vec::new());
    }

    // Step 2: Boards Discovery (Parallel Processing)
    log::info!("[2/2] {}", locale.messages.mapper_step_boards);

    let selector_detector = SelectorDetector::new(seed.cms_patterns.clone());
    let board_service = Arc::new(BoardDiscoveryService::new(
        client,
        seed.keywords.clone(),
        selector_detector,
        &config.discovery,
    ));

    let mut all_manual_reviews: Vec<ManualReviewItem> = Vec::new();

    for campus in &mut campuses {
        log::info!("Processing campus: {}", campus.campus);

        for college in &mut campus.colleges {
            let departments = std::mem::take(&mut college.departments);

            let (processed_depts, reviews): (Vec<_>, Vec<_>) = stream::iter(departments)
                .map(|mut dept| {
                    let service = Arc::clone(&board_service);
                    let campus_name = campus.campus.clone();
                    let show_progress = config.logging.show_progress;
                    let dept_name = dept.name.clone();

                    async move {
                        if show_progress {
                            log::debug!("Scanning: {}", dept_name);
                        }

                        let result = service.discover(&campus_name, &dept.name, &dept.url).await;
                        dept.boards = result.boards;

                        if show_progress {
                            log::debug!("Found {} boards for {}", dept.boards.len(), dept_name);
                        }

                        (dept, result.manual_review)
                    }
                })
                .buffer_unordered(CONCURRENCY_LIMIT)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .unzip();

            college.departments = processed_depts;
            all_manual_reviews.extend(reviews.into_iter().flatten());
        }
    }

    if !all_manual_reviews.is_empty() {
        log::warn!(
            "Found {} items needing manual review",
            all_manual_reviews.len()
        );
    }

    let total_depts: usize = campuses.iter().map(|c| c.department_count()).sum();
    let total_boards: usize = campuses.iter().map(|c| c.board_count()).sum();

    log::info!(
        "Mapper complete: {} departments, {} boards discovered",
        total_depts,
        total_boards
    );

    Ok(campuses)
}
