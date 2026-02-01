//! Notice data structure.
//!
//! Aligned with README.md data schema for Hot/Cold storage pattern.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::NoticeCategory;

/// A notice fetched from a board (internal representation).
///
/// This contains all crawled metadata. For JSON output, convert to `NoticeOutput`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notice {
    /// Campus name
    pub campus: String,

    /// College name (empty string if department is directly under campus)
    pub college: String,

    /// Department unique identifier
    pub department_id: String,

    /// Department display name
    pub department_name: String,

    /// Board unique identifier
    pub board_id: String,

    /// Board display name
    pub board_name: String,

    /// Notice title
    pub title: String,

    /// Notice author
    #[serde(default)]
    pub author: String,

    /// Notice date (YYYY-MM-DD format)
    pub date: String,

    /// Full URL to the notice
    pub link: String,

    /// Optional source-system notice identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,

    /// Whether this notice is pinned/important
    #[serde(default)]
    pub is_pinned: bool,
}

impl Notice {
    /// Compute a canonical identifier for deduplication.
    /// Format: YYYYMMDD-XXX (date-based with sequence number)
    pub fn canonical_id(&self) -> String {
        // Create a hash-based short ID
        let normalized = format!(
            "{}|{}|{}|{}|{}",
            self.campus.trim().to_lowercase(),
            self.department_id.trim().to_lowercase(),
            self.board_id.trim().to_lowercase(),
            self.source_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_lowercase(),
            self.link.trim().to_lowercase()
        );
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        let digest = hasher.finalize();

        // Use date prefix + first 6 hex chars of hash
        let date_part = self.normalized_date().replace("-", "");
        let hash_part = &hex::encode(digest)[..6];
        format!("{}-{}", date_part, hash_part)
    }

    /// Normalize date to YYYY-MM-DD format.
    pub fn normalized_date(&self) -> String {
        // Handle various date formats: YYYY.MM.DD, YYYY-MM-DD, YYYY/MM/DD
        let cleaned = self.date.replace(['.', '/'], "-");

        // Handle 2-digit year (YY-MM-DD -> 20YY-MM-DD)
        let parts: Vec<&str> = cleaned.split('-').collect();
        let cleaned_with_full_year = if parts.len() == 3 && parts[0].len() == 2 {
            // Two-digit year detected, convert to 20YY
            format!("20{}-{}-{}", parts[0], parts[1], parts[2])
        } else {
            cleaned
        };

        // Try to parse and reformat
        if let Ok(date) = NaiveDate::parse_from_str(&cleaned_with_full_year, "%Y-%m-%d") {
            date.format("%Y-%m-%d").to_string()
        } else {
            // Fallback: return as-is with dots replaced
            cleaned_with_full_year
        }
    }

    /// Get the year-month for archiving (YYYY, MM).
    pub fn archive_period(&self) -> (i32, u32) {
        let normalized = self.normalized_date();
        if let Ok(date) = NaiveDate::parse_from_str(&normalized, "%Y-%m-%d") {
            (date.year(), date.month())
        } else {
            // Fallback to current date
            let now = chrono::Utc::now().naive_utc().date();
            (now.year(), now.month())
        }
    }
}

use chrono::Datelike;

/// Output format for JSON files (matches README.md schema).
///
/// ```json
/// {
///   "id": "20260131-001",
///   "title": "2026 Spring Semester Course Registration Guide",
///   "url": "https://univ.edu/notice/12345",
///   "date": "2026-01-31",
///   "category": "academic",
///   "is_pinned": true
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoticeOutput {
    /// Unique identifier (format: YYYYMMDD-XXX)
    pub id: String,

    /// Notice title
    pub title: String,

    /// Full URL to the notice
    pub url: String,

    /// Notice date (YYYY-MM-DD format)
    pub date: String,

    /// Notice category
    pub category: NoticeCategory,

    /// Whether this notice is pinned/important
    pub is_pinned: bool,
}

impl From<&Notice> for NoticeOutput {
    fn from(notice: &Notice) -> Self {
        Self {
            id: notice.canonical_id(),
            title: notice.title.clone(),
            url: notice.link.clone(),
            date: notice.normalized_date(),
            category: super::map_category(&notice.board_name),
            is_pinned: notice.is_pinned,
        }
    }
}

impl From<Notice> for NoticeOutput {
    fn from(notice: Notice) -> Self {
        Self::from(&notice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_notice() -> Notice {
        Notice {
            campus: "TestCampus".to_string(),
            college: "TestCollege".to_string(),
            department_id: "dept1".to_string(),
            department_name: "Department".to_string(),
            board_id: "notice".to_string(),
            board_name: "공지사항".to_string(),
            title: "Test Title".to_string(),
            author: "Admin".to_string(),
            date: "2024-01-15".to_string(),
            link: "https://example.com/notice/1".to_string(),
            source_id: None,
            is_pinned: false,
        }
    }

    #[test]
    fn test_canonical_id_format() {
        let notice = sample_notice();
        let id = notice.canonical_id();
        // Should be YYYYMMDD-XXXXXX format
        assert!(
            id.starts_with("20240115-"),
            "ID should start with date: {}",
            id
        );
        assert_eq!(id.len(), 15, "ID should be 15 chars: YYYYMMDD-XXXXXX");
    }

    #[test]
    fn test_canonical_id_is_stable() {
        let notice = sample_notice();
        let first = notice.canonical_id();
        let second = notice.canonical_id();
        assert_eq!(first, second);
    }

    #[test]
    fn test_normalized_date() {
        let mut notice = sample_notice();

        notice.date = "2024.01.15".to_string();
        assert_eq!(notice.normalized_date(), "2024-01-15");

        notice.date = "2024/01/15".to_string();
        assert_eq!(notice.normalized_date(), "2024-01-15");

        notice.date = "2024-01-15".to_string();
        assert_eq!(notice.normalized_date(), "2024-01-15");
    }

    #[test]
    fn test_archive_period() {
        let notice = sample_notice();
        let (year, month) = notice.archive_period();
        assert_eq!(year, 2024);
        assert_eq!(month, 1);
    }

    #[test]
    fn test_notice_output_conversion() {
        let notice = sample_notice();
        let output: NoticeOutput = (&notice).into();

        assert_eq!(output.title, notice.title);
        assert_eq!(output.url, notice.link);
        assert_eq!(output.date, "2024-01-15");
        assert!(!output.is_pinned);
    }
}
