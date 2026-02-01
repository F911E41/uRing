//! Domain models for the crawler application.
//!
//! This module contains all data structures used throughout the application,
//! organized by their primary purpose.
//!
//! ## Storage Schema (README.md aligned)
//!
//! - Hot Data: `current.json` - Array of recent notices
//! - Cold Data: `stacks/YYYY/MM.json` - Monthly archives

mod campus;
mod config;
mod notice;
mod seed;
mod selectors;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Re-export all public types
pub use campus::{Board, Campus, CampusMeta, College, Department, DepartmentRef};
pub use config::{Config, CrawlerConfig, DiscoveryConfig, LocaleConfig};
pub use notice::{Notice, NoticeOutput};
pub use seed::{CampusInfo, CmsPattern, KeywordMapping, Seed};
pub use selectors::CmsSelectors;

/// Statistics for a crawl session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlStats {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub notice_count: usize,
    pub department_count: usize,
    pub board_count: usize,
    pub board_total: usize,
    pub board_failures: usize,
    pub board_success_rate: f32,
    pub notice_total: usize,
    pub notice_failures: usize,
    pub notice_success_rate: f32,
    pub detail_total: usize,
    pub detail_failures: usize,
    pub detail_success_rate: f32,
}

/// Crawl stage for structured error reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlStage {
    Selector,
    BoardList,
    NoticeDetail,
    BoardLookup,
}

/// Structured crawl error for storage/reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlError {
    pub stage: CrawlStage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice_id: Option<String>,
    pub message: String,
    pub retryable: bool,
}

/// Summary of a crawl run.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrawlOutcome {
    #[serde(default)]
    pub notices: Vec<Notice>,
    pub board_total: usize,
    pub board_failures: usize,
    pub notice_total: usize,
    pub notice_failures: usize,
    pub detail_total: usize,
    pub detail_failures: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<CrawlError>,
}

/// Crawl outcome report without notice payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlOutcomeReport {
    pub board_total: usize,
    pub board_failures: usize,
    pub notice_total: usize,
    pub notice_failures: usize,
    pub detail_total: usize,
    pub detail_failures: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<CrawlError>,
}

impl From<&CrawlOutcome> for CrawlOutcomeReport {
    fn from(outcome: &CrawlOutcome) -> Self {
        Self {
            board_total: outcome.board_total,
            board_failures: outcome.board_failures,
            notice_total: outcome.notice_total,
            notice_failures: outcome.notice_failures,
            detail_total: outcome.detail_total,
            detail_failures: outcome.detail_failures,
            errors: outcome.errors.clone(),
        }
    }
}

/// Represents a notice category.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NoticeCategory {
    Academic,
    Scholarship,
    Recruitment,
    Event,
    General,
    Other,
}

/// Metadata for notice categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMeta {
    pub id: NoticeCategory,
    pub name: String,
}

impl CategoryMeta {
    pub fn all() -> Vec<CategoryMeta> {
        vec![
            CategoryMeta {
                id: NoticeCategory::Academic,
                name: "학사".to_string(),
            },
            CategoryMeta {
                id: NoticeCategory::Scholarship,
                name: "장학".to_string(),
            },
            CategoryMeta {
                id: NoticeCategory::Recruitment,
                name: "채용".to_string(),
            },
            CategoryMeta {
                id: NoticeCategory::Event,
                name: "행사".to_string(),
            },
            CategoryMeta {
                id: NoticeCategory::General,
                name: "일반".to_string(),
            },
            CategoryMeta {
                id: NoticeCategory::Other,
                name: "기타".to_string(),
            },
        ]
    }
}

/// Maps a board name to a notice category.
pub fn map_category(board_name: &str) -> NoticeCategory {
    if board_name.contains("학사") {
        NoticeCategory::Academic
    } else if board_name.contains("장학") {
        NoticeCategory::Scholarship
    } else if board_name.contains("채용") || board_name.contains("취업") {
        NoticeCategory::Recruitment
    } else if board_name.contains("행사") || board_name.contains("안내") {
        NoticeCategory::Event
    } else if board_name.contains("일반") {
        NoticeCategory::General
    } else {
        NoticeCategory::Other
    }
}

/// Represents the difference between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Diff {
    /// Notice IDs that were added in the new snapshot.
    pub added: Vec<String>,

    /// Notice IDs that were updated in the new snapshot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updated: Vec<String>,

    /// Notice IDs that were removed in the new snapshot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
}

/// Result of board discovery for a department.
#[derive(Debug, Default)]
pub struct BoardDiscoveryResult {
    pub boards: Vec<Board>,
    pub manual_review: Option<ManualReviewItem>,
}

/// Represents a department that needs manual review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualReviewItem {
    pub campus: String,
    pub name: String,
    pub url: String,
    pub reason: String,
}
