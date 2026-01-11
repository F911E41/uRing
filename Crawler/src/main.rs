// src/main.rs

//! uRing: Integrated University Notice Crawler CLI
//!
//! This is the main CLI entry point for local development and testing.
//! For AWS Lambda deployment, use the `lambda` binary with the `lambda` feature.

#[cfg(feature = "lambda")]
mod lambda;

mod config;
mod error;
mod models;
mod services;
mod storage;
mod utils;

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{Datelike, Utc};
use clap::{Parser, Subcommand};

use crate::config::load_all;
use crate::error::{AppError, Result};
use crate::models::{Campus, Config, LocaleConfig, ManualReviewItem, Notice, Seed};
use crate::services::{BoardDiscoveryService, DepartmentCrawler, NoticeCrawler, SelectorDetector};
use crate::storage::paths::{monthly_archive_key, monthly_prefix, new_notices_key};
use crate::storage::{LocalStorage, NoticeStorage};
use crate::utils::{
    fs::{ensure_dir, save_json},
    http::create_client,
    log, save_notices,
};

#[derive(Parser, Debug)]
#[command(
    name = "uRing",
    version = "1.0.0",
    about = "Integrated University Notice Crawler"
)]

/// CLI Arguments
struct Cli {
    #[arg(short, long, default_value = "data/config.toml")]
    config: String,

    #[arg(long, default_value = "data/locale.toml")]
    locale: String,

    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Command,
}

/// CLI Commands
#[derive(Subcommand, Debug)]
enum Command {
    /// Discover departments and boards
    Map {
        #[arg(long)]
        force: bool,
    },
    /// Fetch notices from discovered boards
    Crawl {
        #[arg(long)]
        site_map: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Validate configuration and seed data
    Validate,
    /// Archive new notices to monthly storage
    Archive,
    /// Load notices from storage
    Load {
        /// Load from "new" storage or specific month (YYYY-MM format)
        #[arg(long, default_value = "new")]
        from: String,
    },
    /// Run the full pipeline
    Pipeline {
        /// Skip the map step (use existing sitemap)
        #[arg(long)]
        skip_map: bool,
    },
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut config = Config::load_or_default(&cli.config);
    let locale = LocaleConfig::load_or_default(&cli.locale);

    // Initialize logging system
    log::init(&locale, &config.logging.level);

    if cli.quiet {
        config.output.console_enabled = false;
        config.logging.show_progress = false;
    }

    match cli.command {
        Command::Map { force: _ } => run_mapper(&config, &locale, &base_path).await?,
        Command::Crawl { site_map, output } => {
            if let Some(path) = site_map {
                config.paths.departments_boards_file = path;
            }
            if let Some(path) = output {
                config.paths.output_dir = path;
            }
            run_crawler(&config, &locale, &base_path).await?;
        }
        Command::Validate => run_validate(&locale, &base_path)?,
        Command::Archive => run_archive(&config, &locale).await?,
        Command::Load { from } => run_load(&config, &locale, &from).await?,
        Command::Pipeline { skip_map } => {
            run_pipeline(&config, &locale, &base_path, skip_map).await?
        }
    }

    Ok(())
}

/// Archive new notices to monthly storage.
async fn run_archive(config: &Config, locale: &LocaleConfig) -> Result<()> {
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

/// Load notices from storage.
async fn run_load(config: &Config, locale: &LocaleConfig, from: &str) -> Result<()> {
    let storage = LocalStorage::new(&config.paths.output);

    let notices = if from == "new" {
        log::info(&locale.messages.load_new);
        storage.load_new().await?
    } else {
        // Parse YYYY-MM format
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

/// Validate configuration and seed data using load_all.
fn run_validate(locale: &LocaleConfig, base_path: &PathBuf) -> Result<()> {
    log::header(&locale.messages.validate_starting);

    match load_all(base_path) {
        Ok((config, seed)) => {
            log::success(&locale.messages.validate_config_success);
            log::sub_item(
                &locale
                    .messages
                    .validate_user_agent
                    .replace("{value}", &config.crawler.user_agent),
            );
            log::sub_item(
                &locale
                    .messages
                    .validate_timeout
                    .replace("{value}", &config.crawler.timeout_secs.to_string()),
            );
            log::sub_item(
                &locale
                    .messages
                    .validate_max_concurrent
                    .replace("{value}", &config.crawler.max_concurrent.to_string()),
            );

            log::success(&locale.messages.validate_seed_success);
            log::sub_item(
                &locale
                    .messages
                    .validate_campuses
                    .replace("{count}", &seed.campuses.len().to_string()),
            );
            log::sub_item(
                &locale
                    .messages
                    .validate_keywords
                    .replace("{count}", &seed.keywords.len().to_string()),
            );
            log::sub_item(
                &locale
                    .messages
                    .validate_patterns
                    .replace("{count}", &seed.cms_patterns.len().to_string()),
            );
            Ok(())
        }
        Err(e) => {
            log::error(
                &locale
                    .messages
                    .validate_failed
                    .replace("{error}", &e.to_string()),
            );
            Err(e)
        }
    }
}

/// Run the mapper to discover departments and boards.
async fn run_mapper(config: &Config, locale: &LocaleConfig, base_path: &PathBuf) -> Result<()> {
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

        // Validate seed data
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

/// Run the notice crawler.
async fn run_crawler(config: &Config, locale: &LocaleConfig, base_path: &PathBuf) -> Result<()> {
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

    // Store notices using LocalStorage (demonstrates NoticeStorage trait usage)
    let storage = LocalStorage::new(&config.paths.output);
    let metadata = storage.store_new(&notices).await?;

    log::success(
        &locale
            .messages
            .storage_saved
            .replace("{count}", &metadata.notice_count.to_string())
            .replace("{path}", &metadata.location),
    );

    // Display S3 storage paths that would be used in Lambda mode
    if config.logging.show_progress {
        let bucket_prefix = "uRing";
        let now = Utc::now();
        log::info(&locale.messages.storage_paths_header);
        log::sub_item(&format!("New notices: {}", new_notices_key(bucket_prefix)));
        log::sub_item(&format!(
            "Archive: {}",
            monthly_archive_key(bucket_prefix, now)
        ));
        log::sub_item(&format!(
            "Monthly prefix: {}",
            monthly_prefix(bucket_prefix, now.year(), now.month())
        ));
    }

    log::success(&locale.messages.crawler_complete);

    Ok(())
}

/// Run the full pipeline
async fn run_pipeline(
    config: &Config,
    locale: &LocaleConfig,
    base_path: &PathBuf,
    skip_map: bool,
) -> Result<()> {
    log::header(&locale.messages.pipeline_starting);

    let total_steps = if skip_map { 2 } else { 3 };
    let mut current_step = 1;

    // Step 1: Map (unless skipped)
    if !skip_map {
        log::step(
            current_step,
            total_steps,
            "Map - Discovering departments and boards",
        );
        run_mapper(config, locale, base_path).await?;
        current_step += 1;
    }

    // Step 2: Crawl
    log::step(current_step, total_steps, "Crawl - Fetching notices");
    run_crawler(config, locale, base_path).await?;
    current_step += 1;

    // Step 3: Archive
    log::step(current_step, total_steps, "Archive - Storing notices");
    run_archive(config, locale).await?;

    log::success(&locale.messages.pipeline_complete);

    Ok(())
}

/// Print notices to console in formatted manner.
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
