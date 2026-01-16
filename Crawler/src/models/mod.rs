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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Re-export all public types
pub use campus::{Board, Campus, CampusMeta, College, Department, DepartmentRef};
pub use config::{Config, CrawlerConfig, DiscoveryConfig, LocaleConfig};
pub use notice::Notice;
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
    pub success_rate: f32,
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

/// A summarized notice for index files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoticeIndexItem {
    pub id: String,
    pub title: String,
    pub date: String,
    pub link: String,
    pub department_name: String,
    pub board_name: String,
    pub category: NoticeCategory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

impl From<&Notice> for NoticeIndexItem {
    fn from(notice: &Notice) -> Self {
        Self {
            id: notice.canonical_id(),
            title: notice.title.clone(),
            date: notice.date.clone(),
            link: notice.link.clone(),
            department_name: notice.department_name.clone(),
            board_name: notice.board_name.clone(),
            category: map_category(&notice.board_name),
            content_hash: Some(notice.content_hash()),
        }
    }
}
