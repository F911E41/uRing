// src/pipeline/map.rs

use std::sync::Arc;

use futures::{StreamExt, stream};
use reqwest::Client;

use crate::error::Result;
use crate::models::{Campus, Config, LocaleConfig, ManualReviewItem, Seed};
use crate::services::{BoardDiscoveryService, DepartmentCrawler, SelectorDetector};
use crate::utils::log;

/// Maximum concurrency for board discovery.
const CONCURRENCY_LIMIT: usize = 14;

/// Run the mapper to discover departments and boards.
pub async fn run_mapper(
    config: &Config,
    locale: &LocaleConfig,
    seed: &Seed,
    client: &Client,
) -> Result<Vec<Campus>> {
    log::header(&locale.messages.mapper_starting);

    seed.validate()?;
    log::success(
        &locale
            .messages
            .mapper_loaded_campuses
            .replace("{count}", &seed.campuses.len().to_string()),
    );

    // Departments Discovery
    log::step(1, 2, &locale.messages.mapper_step_departments);

    let dept_crawler = DepartmentCrawler::new(client);
    let mut campuses = dept_crawler.crawl_all(&seed.campuses).await?;

    if campuses.is_empty() {
        log::error(
            &locale
                .messages
                .dept_failed
                .replace("{name}", "any campus")
                .replace("{error}", "No results"),
        );
        return Ok(Vec::new());
    }

    // Boards Discovery (Parallel Processing with Controlled Concurrency)
    log::step(2, 2, &locale.messages.mapper_step_boards);

    let selector_detector = SelectorDetector::new(seed.cms_patterns.clone());

    // Make the service shareable across async tasks using Arc
    let board_service = Arc::new(BoardDiscoveryService::new(
        client,
        seed.keywords.clone(),
        selector_detector,
        &config.discovery,
    ));

    let mut all_manual_reviews: Vec<ManualReviewItem> = Vec::new();

    for campus in &mut campuses {
        log::info(
            &locale
                .messages
                .mapper_campus
                .replace("{name}", &campus.campus),
        );

        for college in &mut campus.colleges {
            // Temporarily take ownership of the Departments within the College.
            let departments = std::mem::take(&mut college.departments);

            // Perform parallel processing using Stream
            let (processed_depts, reviews): (Vec<_>, Vec<_>) = stream::iter(departments)
                .map(|mut dept| {
                    // Clone data for each task (Arc clone is cheap)
                    let service = Arc::clone(&board_service);
                    let campus_name = campus.campus.clone();
                    let show_progress = config.logging.show_progress;

                    // Clone logging messages (for move into async block)
                    let msg_scanning = locale.messages.mapper_dept_scanning.clone();
                    let msg_found = locale.messages.mapper_dept_found_boards.clone();

                    async move {
                        if show_progress {
                            log::debug(&msg_scanning.replace("{name}", &dept.name));
                        }

                        // Actual discovery (asynchronous)
                        let result = service.discover(&campus_name, &dept.name, &dept.url).await;

                        dept.boards = result.boards;

                        if show_progress {
                            log::info(
                                &msg_found.replace("{count}", &dept.boards.len().to_string()),
                            );
                        }

                        // Return a tuple of (processed department, review items)
                        (dept, result.manual_review)
                    }
                })
                .buffer_unordered(CONCURRENCY_LIMIT) // Run N tasks concurrently
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .unzip(); // Seperate processed departments and reviews

            // Reassign the processed departments list
            college.departments = processed_depts;

            // Collect review items into a single vector
            all_manual_reviews.extend(reviews.into_iter().flatten());
        }
    }

    // Log summary of manual reviews
    if !all_manual_reviews.is_empty() {
        log::warn(&format!(
            "Found {} items needing manual review.",
            all_manual_reviews.len()
        ));
    }

    let total_depts: usize = campuses.iter().map(|c| c.department_count()).sum();
    let total_boards: usize = campuses.iter().map(|c| c.board_count()).sum();
    log::summary(
        "Mapper Results",
        &[
            (
                &locale.messages.summary_total_depts,
                total_depts.to_string(),
            ),
            (
                &locale.messages.summary_total_boards,
                total_boards.to_string(),
            ),
            (
                &locale.messages.summary_manual_review,
                all_manual_reviews.len().to_string(),
            ),
        ],
    );

    Ok(campuses)
}
