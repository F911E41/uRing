// src/pipeline/map.rs

use std::path::PathBuf;

use crate::error::{AppError, Result};
use crate::models::{Config, LocaleConfig, ManualReviewItem, Seed};
use crate::services::{BoardDiscoveryService, DepartmentCrawler, SelectorDetector};
use crate::utils::{
    fs::{ensure_dir, save_json},
    http::create_client,
    log,
};

/// Run the mapper to discover departments and boards.
pub async fn run_mapper(config: &Config, locale: &LocaleConfig, base_path: &PathBuf) -> Result<()> {
    let config = config.clone();
    let base_path = base_path.clone();
    let locale = locale.clone();

    tokio::task::spawn_blocking(move || {
        log::header(&locale.messages.mapper_starting);

        let seed_path = config.seed_path(&base_path);
        log::info(
            &locale
                .messages
                .mapper_loading_seed
                .replace("{path}", &format!("{:?}", seed_path)),
        );
        let seed = Seed::load(&seed_path).map_err(|e| {
            AppError::config(format!(
                "{}: {}",
                locale
                    .errors
                    .seed_load_failed
                    .replace("{path}", &format!("{:?}", seed_path))
                    .replace("{error}", ""),
                e
            ))
        })?;

        seed.validate().map_err(|e| {
            AppError::validation(format!(
                "{}: {}",
                locale.errors.seed_validation_failed.replace("{error}", ""),
                e
            ))
        })?;
        log::success(
            &locale
                .messages
                .mapper_loaded_campuses
                .replace("{count}", &seed.campuses.len().to_string()),
        );

        ensure_dir(&config.output_dir(&base_path))?;
        let client = create_client(&config.crawler)?;

        log::step(1, 2, &locale.messages.mapper_step_departments);

        let dept_crawler = DepartmentCrawler::new(&client);
        let mut campuses = dept_crawler.crawl_all(&seed.campuses)?;

        if campuses.is_empty() {
            log::error(
                &locale
                    .messages
                    .dept_failed
                    .replace("{name}", "any campus")
                    .replace("{error}", "No results"),
            );
            return Ok(());
        }

        let dept_path = config.departments_path(&base_path);
        save_json(&dept_path, &campuses)?;
        log::success(&format!("Saved initial departments to {:?}", dept_path));

        log::step(2, 2, &locale.messages.mapper_step_boards);

        let selector_detector = SelectorDetector::new(seed.cms_patterns.clone());
        let board_service = BoardDiscoveryService::new(
            &client,
            seed.keywords.clone(),
            selector_detector,
            &config.discovery,
            config.crawler.sitemap_timeout_secs,
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
                    let result = board_service.discover(&campus.campus, &dept.name, &dept.url);
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

        let boards_path = config.departments_boards_path(&base_path);
        save_json(&boards_path, &campuses)?;
        log::success(
            &locale
                .messages
                .mapper_complete
                .replace("{path}", &format!("{:?}", boards_path)),
        );

        if !manual_review_items.is_empty() {
            let review_path = config.manual_review_path(&base_path);
            save_json(&review_path, &manual_review_items)?;
            log::warn(
                &locale
                    .messages
                    .mapper_manual_review
                    .replace("{count}", &manual_review_items.len().to_string())
                    .replace("{path}", &format!("{:?}", review_path)),
            );
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

        Ok(())
    })
    .await
    .map_err(|e| AppError::config(format!("Task execution failed: {}", e)))?
}
