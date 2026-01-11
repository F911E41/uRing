// src/models/mod.rs

//! Domain models for the crawler application.
//!
//! This module contains all data structures used throughout the application,
//! organized by their primary purpose.

mod campus;
mod config;
mod notice;
mod seed;
mod selectors;

// Re-export all public types
pub use campus::{Board, Campus, College, Department, DepartmentRef};
pub use config::{Config, CrawlerConfig, DiscoveryConfig, LocaleConfig};
pub use notice::Notice;
pub use seed::{CampusInfo, CmsPattern, KeywordMapping, Seed};
pub use selectors::CmsSelectors;

/// Result of board discovery for a department.
#[derive(Debug, Default)]
pub struct BoardDiscoveryResult {
    pub boards: Vec<Board>,
    pub manual_review: Option<ManualReviewItem>,
}

/// Represents a department that needs manual review.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManualReviewItem {
    pub campus: String,
    pub name: String,
    pub url: String,
    pub reason: String,
}
