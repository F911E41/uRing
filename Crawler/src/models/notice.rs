// src/models/notice.rs

//! Notice data structure.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A notice fetched from a board.
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
    pub author: String,

    /// Notice date
    pub date: String,

    /// Full URL to the notice
    pub link: String,

    /// Optional source-system notice identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,

    /// Notice body content (HTML or text)
    #[serde(default)]
    pub body: String,
}

impl Notice {
    /// Compute a canonical identifier for deduplication.
    pub fn canonical_id(&self) -> String {
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
        hex::encode(digest)
    }

    /// Compute a content hash for update detection.
    pub fn content_hash(&self) -> String {
        let normalized = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.campus.trim(),
            self.college.trim(),
            self.department_id.trim(),
            self.department_name.trim(),
            self.board_id.trim(),
            self.board_name.trim(),
            self.title.trim(),
            self.author.trim(),
            self.date.trim(),
            self.link.trim(),
            self.body.trim()
        );
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        let digest = hasher.finalize();
        hex::encode(digest)
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
            date: "2024-01-01".to_string(),
            link: "https://example.com/notice/1".to_string(),
            source_id: None,
            body: "<p>Hello, world!</p>".to_string(),
        }
    }

    #[test]
    fn test_canonical_id_is_stable() {
        let notice = sample_notice();
        let first = notice.canonical_id();
        let second = notice.canonical_id();
        assert_eq!(first, second);
    }
}
