//! Mapper - Yonsei University Department & Board Crawler
//!
//! This program crawls Yonsei University websites to discover announcement
//! boards for each department.

mod config;
mod error;
mod models;
mod services;
mod utils;

use std::path::PathBuf;

use error::Result;
use models::ManualReviewItem;
use services::{BoardDiscoveryService, DepartmentCrawler, SelectorDetector};

use utils::fs::{ensure_dir, save_json};
use utils::http::create_client;

fn get_base_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn main() -> Result<()> {
    let base_path = get_base_path();

    // Load configuration
    println!("Loading configuration...");
    let (config, seed) = config::load_all(&base_path)?;

    println!("  Loaded {} campuses", seed.campuses.len());
    println!("  Loaded {} keywords", seed.keywords.len());
    println!("  Loaded {} CMS patterns", seed.cms_patterns.len());

    // Ensure output directory exists
    ensure_dir(&config.output_dir(&base_path))?;

    // Create HTTP client
    let client = create_client(&config.http)?;

    // Step 1: Crawl departments
    println!("\n{}", "=".repeat(60));
    println!("Step 1: Crawling departments...");
    println!("{}", "=".repeat(60));

    let crawler = DepartmentCrawler::new(&client);
    let mut campuses = crawler.crawl_all(&seed.campuses)?;

    if campuses.is_empty() {
        println!("Failed to crawl any campus. Exiting.");
        return Ok(());
    }

    // Save initial department data
    let dept_path = config.departments_path(&base_path);
    save_json(&dept_path, &campuses)?;
    println!("\nSaved department data to {:?}", dept_path);

    // Step 2: Discover boards
    println!("\n{}", "=".repeat(60));
    println!("Step 2: Discovering boards...");
    println!("{}", "=".repeat(60));

    let selector_detector = SelectorDetector::new(seed.cms_patterns);
    let board_service =
        BoardDiscoveryService::new(&client, seed.keywords, selector_detector, &config);

    let mut manual_review_items: Vec<ManualReviewItem> = Vec::new();

    for campus in &mut campuses {
        println!("\n[{}]", campus.campus);

        for college in &mut campus.colleges {
            for dept in &mut college.departments {
                if config.logging.show_progress {
                    println!("  {}...", dept.name);
                }

                let result = board_service.discover(&campus.campus, &dept.name, &dept.url);

                dept.boards = result.boards;

                if let Some(review_item) = result.manual_review {
                    manual_review_items.push(review_item);
                }

                if !dept.boards.is_empty() && config.logging.show_progress {
                    println!("    Found {} board(s)", dept.boards.len());
                }
            }
        }
    }

    // Save results
    let boards_path = config.departments_boards_path(&base_path);
    save_json(&boards_path, &campuses)?;
    println!("\nSaved departments with boards to {:?}", boards_path);

    let review_path = config.manual_review_path(&base_path);
    save_json(&review_path, &manual_review_items)?;
    println!(
        "Saved {} items needing manual review to {:?}",
        manual_review_items.len(),
        review_path
    );

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("Summary");
    println!("{}", "=".repeat(60));

    let total_depts: usize = campuses
        .iter()
        .flat_map(|c| &c.colleges)
        .map(|col| col.departments.len())
        .sum();

    let total_boards: usize = campuses
        .iter()
        .flat_map(|c| &c.colleges)
        .flat_map(|col| &col.departments)
        .map(|d| d.boards.len())
        .sum();

    println!("  Total departments: {}", total_depts);
    println!("  Total boards found: {}", total_boards);
    println!("  Needs manual review: {}", manual_review_items.len());

    Ok(())
}
