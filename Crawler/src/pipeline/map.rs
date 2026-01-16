// src/pipeline/map.rs

use reqwest::Client;

use crate::error::Result;
use crate::models::{Campus, Config, LocaleConfig, ManualReviewItem, Seed};
use crate::services::{BoardDiscoveryService, DepartmentCrawler, SelectorDetector};
use crate::utils::log;

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

    log::step(2, 2, &locale.messages.mapper_step_boards);

    let selector_detector = SelectorDetector::new(seed.cms_patterns.clone());
    let board_service = BoardDiscoveryService::new(
        client,
        seed.keywords.clone(),
        selector_detector,
        &config.discovery,
    );

    let mut manual_review_items: Vec<ManualReviewItem> = Vec::new();

    for campus in &mut campuses {
        log::info(
            &locale
                .messages
                .mapper_campus
                .replace("{name}", &campus.campus),
        );
        for college in &mut campus.colleges {
            for dept in &mut college.departments {
                if config.logging.show_progress {
                    log::debug(
                        &locale
                            .messages
                            .mapper_dept_scanning
                            .replace("{name}", &dept.name),
                    );
                }
                let result = board_service
                    .discover(&campus.campus, &dept.name, &dept.url)
                    .await;
                dept.boards = result.boards;
                if let Some(review_item) = result.manual_review {
                    manual_review_items.push(review_item);
                }
                if config.logging.show_progress {
                    log::info(
                        &locale
                            .messages
                            .mapper_dept_found_boards
                            .replace("{count}", &dept.boards.len().to_string()),
                    );
                }
            }
        }
    }

    // Manual review items are logged but not saved to a file in this version
    if !manual_review_items.is_empty() {
        log::warn(&format!(
            "Found {} items needing manual review.",
            manual_review_items.len()
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
                manual_review_items.len().to_string(),
            ),
        ],
    );

    Ok(campuses)
}
