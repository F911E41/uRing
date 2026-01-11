// src/main.rs
//! uRing: Integrated University Notice Crawler CLI
//!
//! This is the main CLI entry point for local development and testing.
//! For AWS Lambda deployment, use the `lambda` binary with the `lambda` feature.

mod config;
mod error;
#[cfg(feature = "lambda")]
mod lambda;
mod models;
mod services;
mod storage;
mod utils;

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};

use crate::error::Result;
use crate::models::{Campus, Config, LocaleConfig, ManualReviewItem, Notice, Seed};
use crate::services::{BoardDiscoveryService, DepartmentCrawler, NoticeCrawler, SelectorDetector};
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

#[derive(Subcommand, Debug)]
enum Command {
    /// Step 1: Discover departments and boards
    Map {
        #[arg(long)]
        force: bool,
    },
    /// Step 2: Fetch notices from discovered boards
    Crawl {
        #[arg(long)]
        site_map: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut config = Config::load_or_default(&cli.config);
    let locale = LocaleConfig::load_or_default(&cli.locale);

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
    }

    Ok(())
}

async fn run_mapper(config: &Config, base_path: &PathBuf) -> Result<()> {
    println!("🗺️  Starting Mapper Mode...");

    let seed_path = config.seed_path(base_path);
    println!("   Loading seed data from {seed_path:?}");
    let seed = Seed::load(&seed_path)?;
    println!("   Loaded {} campuses", seed.campuses.len());

    ensure_dir(&config.output_dir(base_path))?;
    let client = create_client(&config.crawler)?;

    println!(
        "\\n{}\\nStep 1: Crawling departments...\\n{}",
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

async fn run_crawler(config: &Config, locale: &LocaleConfig, base_path: &PathBuf) -> Result<()> {
    if config.logging.show_progress {
        print!("{}", locale.messages.crawler_starting);
    }

    let sitemap_path = config.departments_boards_path(base_path);
    if !sitemap_path.exists() {
        eprintln!("❌ Sitemap not found at {sitemap_path:?}.");
        eprintln!("   Please run 'uRing map' first to generate it.");
        std::process::exit(1);
    }

    let campuses = Campus::load_all(&sitemap_path)?;
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

    Ok(())
}

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
