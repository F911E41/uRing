//! Mapper - Yonsei University Department & Board Crawler
//!
//! This program crawls Yonsei University websites to discover announcement
//! boards for each department.

mod config;
mod crawlers;
mod error;
mod http;
mod models;
mod selectors;

use std::fs;

use config::{data_dir, departments_boards_file, departments_file, manual_review_file};
use crawlers::{crawl_all_campuses, discover_boards};
use error::Result;
use http::create_client;
use models::ManualReviewItem;

fn save_json<T: serde::Serialize>(path: &std::path::Path, data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    fs::write(path, json)?;
    Ok(())
}

fn main() -> Result<()> {
    // Ensure data directory exists
    fs::create_dir_all(data_dir())?;

    let client = create_client()?;

    // Step 1: Crawl departments
    println!("{}", "=".repeat(60));
    println!("Step 1: Crawling Yonsei departments...");
    println!("{}", "=".repeat(60));

    let mut campuses = crawl_all_campuses(&client)?;

    if campuses.is_empty() {
        println!("Failed to crawl any campus. Exiting.");
        return Ok(());
    }

    // Save initial department data
    save_json(&departments_file(), &campuses)?;
    println!("\nSaved department data to {:?}", departments_file());

    // Step 2: Discover boards
    println!("\n{}", "=".repeat(60));
    println!("Step 2: Discovering boards for each department...");
    println!("{}", "=".repeat(60));

    let mut manual_review_items: Vec<ManualReviewItem> = Vec::new();

    for campus in &mut campuses {
        println!("\n[{}]", campus.campus);

        for college in &mut campus.colleges {
            for dept in &mut college.departments {
                println!("  {}...", dept.name);

                let result = discover_boards(&client, &campus.campus, &dept.name, &dept.url);

                dept.boards = result.boards;

                if let Some(review_item) = result.manual_review {
                    manual_review_items.push(review_item);
                }

                if !dept.boards.is_empty() {
                    println!("    Found {} board(s)", dept.boards.len());
                }
            }
        }
    }

    // Save results
    save_json(&departments_boards_file(), &campuses)?;
    println!(
        "\nSaved departments with boards to {:?}",
        departments_boards_file()
    );

    save_json(&manual_review_file(), &manual_review_items)?;
    println!(
        "Saved {} items needing manual review to {:?}",
        manual_review_items.len(),
        manual_review_file()
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
