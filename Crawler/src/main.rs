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
    save_notices,
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
}

/// Initialize logging based on configuration level.
fn init_logging(level: &str) {
    match level {
        "debug" => eprintln!("[DEBUG] Logging initialized at debug level"),
        "info" => {} // Default, no message
        "warn" => eprintln!("[WARN] Logging initialized at warn level"),
        "error" => eprintln!("[ERROR] Logging initialized at error level"),
        _ => eprintln!("[WARN] Unknown log level '{}', using default", level),
    }
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut config = Config::load_or_default(&cli.config);
    let locale = LocaleConfig::load_or_default(&cli.locale);

    // Initialize logging based on config level
    init_logging(&config.logging.level);

    if cli.quiet {
        config.output.console_enabled = false;
        config.logging.show_progress = false;
    }

    match cli.command {
        Command::Map { force: _ } => run_mapper(&config, &base_path).await?,
        Command::Crawl { site_map, output } => {
            if let Some(path) = site_map {
                config.paths.departments_boards_file = path;
            }
            if let Some(path) = output {
                config.paths.output_dir = path;
            }
            run_crawler(&config, &locale, &base_path).await?;
        }
        Command::Validate => run_validate(&base_path)?,
        Command::Archive => run_archive(&config).await?,
        Command::Load { from } => run_load(&config, &from).await?,
    }

    Ok(())
}

/// Archive new notices to monthly storage.
async fn run_archive(config: &Config) -> Result<()> {
    println!("📦 Archiving notices...");
    let storage = LocalStorage::new(&config.paths.output);
    let metadata = storage.rotate_to_archive().await?;

    println!("✅ Archived {} notices", metadata.notice_count);
    println!("   Location: {}", metadata.location);
    println!("   Timestamp: {}", metadata.timestamp);

    Ok(())
}

/// Load notices from storage.
async fn run_load(config: &Config, from: &str) -> Result<()> {
    let storage = LocalStorage::new(&config.paths.output);

    let notices = if from == "new" {
        println!("📂 Loading new notices...");
        storage.load_new().await?
    } else {
        // Parse YYYY-MM format
        let parts: Vec<&str> = from.split('-').collect();
        if parts.len() != 2 {
            return Err(AppError::validation(
                "Invalid date format. Use YYYY-MM (e.g., 2025-01)",
            ));
        }
        let year: i32 = parts[0]
            .parse()
            .map_err(|_| AppError::validation("Invalid year"))?;
        let month: u32 = parts[1]
            .parse()
            .map_err(|_| AppError::validation("Invalid month"))?;

        println!("📂 Loading notices from {}-{:02}...", year, month);
        storage.load_archive(year, month).await?
    };

    println!("✅ Loaded {} notices", notices.len());
    for notice in &notices {
        println!("   - {} [{}]", notice.title, notice.date);
    }

    Ok(())
}

/// Validate configuration and seed data using load_all.
fn run_validate(base_path: &PathBuf) -> Result<()> {
    println!("🔍 Validating configuration and seed data...");

    match load_all(base_path) {
        Ok((config, seed)) => {
            println!("✅ Configuration loaded successfully:");
            println!("   - User agent: {}", config.crawler.user_agent);
            println!("   - Timeout: {}s", config.crawler.timeout_secs);
            println!("   - Max concurrent: {}", config.crawler.max_concurrent);
            println!("✅ Seed data validated:");
            println!("   - Campuses: {}", seed.campuses.len());
            println!("   - Keywords: {}", seed.keywords.len());
            println!("   - CMS patterns: {}", seed.cms_patterns.len());
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Validation failed: {}", e);
            Err(e)
        }
    }
}

/// Run the mapper to discover departments and boards.
async fn run_mapper(config: &Config, base_path: &PathBuf) -> Result<()> {
    println!("🗺️  Starting Mapper Mode...");

    let seed_path = config.seed_path(base_path);
    println!("   Loading seed data from {seed_path:?}");
    let seed = Seed::load(&seed_path)
        .map_err(|e| AppError::config(format!("Failed to load seed data: {}", e)))?;

    // Validate seed data
    seed.validate()
        .map_err(|e| AppError::validation(format!("Seed validation failed: {}", e)))?;
    println!("   Loaded and validated {} campuses", seed.campuses.len());

    ensure_dir(&config.output_dir(base_path))?;
    let client = create_client(&config.crawler)?;

    println!(
        "\\n{}\\nCrawling departments...\\n{}",
        "=".repeat(60),
        "=".repeat(60)
    );

    let dept_crawler = DepartmentCrawler::new(&client);
    let mut campuses = dept_crawler.crawl_all(&seed.campuses)?;

    if campuses.is_empty() {
        eprintln!("❌ Failed to crawl any campus.");
        return Ok(());
    }

    let dept_path = config.departments_path(base_path);
    save_json(&dept_path, &campuses)?;
    println!("✅ Saved initial departments to {dept_path:?}");

    println!(
        "\\n{}\\nStep 2: Discovering boards...\\n{}",
        "=".repeat(60),
        "=".repeat(60)
    );

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
        println!("\\n🏫 [{}]", campus.campus);
        for college in &mut campus.colleges {
            for dept in &mut college.departments {
                if config.logging.show_progress {
                    print!("   🔍 {}... ", dept.name);
                }
                let result = board_service.discover(&campus.campus, &dept.name, &dept.url);
                dept.boards = result.boards;
                if let Some(review_item) = result.manual_review {
                    manual_review_items.push(review_item);
                }
                if config.logging.show_progress {
                    println!("Found {} board(s)", dept.boards.len());
                }
            }
        }
    }

    let boards_path = config.departments_boards_path(base_path);
    save_json(&boards_path, &campuses)?;
    println!("\\n✨ Mapper Complete! Data saved to {boards_path:?}");

    if !manual_review_items.is_empty() {
        let review_path = config.manual_review_path(base_path);
        save_json(&review_path, &manual_review_items)?;
        println!(
            "⚠️  Saved {} items needing manual review to {review_path:?}",
            manual_review_items.len()
        );
    }

    let total_depts: usize = campuses.iter().map(|c| c.department_count()).sum();
    let total_boards: usize = campuses.iter().map(|c| c.board_count()).sum();
    println!("\\n📊 Summary");
    println!("   Total Departments: {total_depts}");
    println!("   Total Boards Found: {total_boards}");
    println!("   Needs Manual Review: {}", manual_review_items.len());

    Ok(())
}

/// Run the notice crawler.
async fn run_crawler(config: &Config, locale: &LocaleConfig, base_path: &PathBuf) -> Result<()> {
    if config.logging.show_progress {
        print!("{}", locale.messages.crawler_starting);
    }

    let sitemap_path = config.departments_boards_path(base_path);
    if !sitemap_path.exists() {
        // Use locale.errors for error message
        let error_msg = format!(
            "{}: {:?}. Please run 'uRing map' first.",
            locale.errors.config_load_failed, sitemap_path
        );
        return Err(AppError::discovery(error_msg));
    }

    let campuses =
        Campus::load_all(&sitemap_path).map_err(|e| AppError::crawl("loading sitemap", e))?;
    let total_depts: usize = campuses.iter().map(|c| c.department_count()).sum();
    let total_boards: usize = campuses.iter().map(|c| c.board_count()).sum();

    if config.logging.show_progress {
        println!(
            "{}",
            locale
                .messages
                .loaded_departments
                .replace("{count_dept}", &total_depts.to_string())
                .replace("{count_board}", &total_boards.to_string())
        );
    }

    let crawler = NoticeCrawler::new(Arc::new(config.clone()));
    let notices = crawler.fetch_all(&campuses).await?;

    print_notices(&notices, config, locale);
    save_notices(&notices, config, locale)?;

    // Store notices using LocalStorage (demonstrates NoticeStorage trait usage)
    let storage = LocalStorage::new(&config.paths.output);
    let metadata = storage.store_new(&notices).await?;
    if config.logging.show_progress {
        println!(
            "\n💾 Storage: {} notices saved to {}",
            metadata.notice_count, metadata.location
        );
    }

    // Display S3 storage paths that would be used in Lambda mode
    if config.logging.show_progress {
        let bucket_prefix = "uRing";
        let now = Utc::now();
        println!("\n📂 S3 Storage Paths (for Lambda deployment):");
        println!("   New notices: {}", new_notices_key(bucket_prefix));
        println!("   Archive: {}", monthly_archive_key(bucket_prefix, now));
        println!(
            "   Monthly prefix: {}",
            monthly_prefix(bucket_prefix, now.year(), now.month())
        );
    }

    Ok(())
}

/// Print notices to console in formatted manner.
fn print_notices(notices: &[Notice], config: &Config, locale: &LocaleConfig) {
    if !config.output.console_enabled {
        return;
    }

    println!(
        "\\n{}",
        locale
            .messages
            .total_notices
            .replace("{total_count}", &notices.len().to_string())
    );
    println!("{:=<80}", locale.messages.separator_line);

    for notice in notices {
        println!("{}", notice.format(&config.output.notice_format));
        println!("{:-<80}", locale.messages.separator_short);
    }
}
