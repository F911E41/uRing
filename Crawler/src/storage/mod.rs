// src/storage/mod.rs

//! Storage abstractions for notice persistence.
//!
//! This module provides a unified interface for storing notices,
//! with implementations for local filesystem and AWS S3.

#[cfg(feature = "lambda")]
pub mod s3;

use std::future::Future;
use std::path::PathBuf;

use chrono::{DateTime, Datelike, Utc};

use crate::error::Result;
use crate::models::Notice;

/// Metadata about a storage operation.
#[derive(Debug, Clone)]
pub struct StorageMetadata {
    /// Number of notices stored
    pub notice_count: usize,
    /// Timestamp of the operation
    pub timestamp: DateTime<Utc>,
    /// Storage location (path or S3 key)
    pub location: String,
}

impl StorageMetadata {
    /// Create new storage metadata
    pub fn new(notice_count: usize, location: impl Into<String>) -> Self {
        Self {
            notice_count,
            timestamp: Utc::now(),
            location: location.into(),
        }
    }
}

/// Trait for notice storage backends.
pub trait NoticeStorage {
    /// Store notices in the "New" directory (hot data for notifications).
    fn store_new(&self, notices: &[Notice])
    -> impl Future<Output = Result<StorageMetadata>> + Send;

    /// Rotate notices from "New" to monthly archive.
    fn rotate_to_archive(&self) -> impl Future<Output = Result<StorageMetadata>> + Send;

    /// Load notices from the "New" directory.
    fn load_new(&self) -> impl Future<Output = Result<Vec<Notice>>> + Send;

    /// Load notices from a specific month's archive.
    fn load_archive(
        &self,
        year: i32,
        month: u32,
    ) -> impl Future<Output = Result<Vec<Notice>>> + Send;
}

/// Local filesystem storage implementation.
pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    /// Create a new local storage with the given base path.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    fn new_notices_path(&self) -> PathBuf {
        self.base_path.join("New").join("notices.json")
    }

    fn archive_path(&self, year: i32, month: u32) -> PathBuf {
        self.base_path
            .join(format!("{:04}-{:02}", year, month))
            .join("notices.json")
    }
}

impl NoticeStorage for LocalStorage {
    async fn store_new(&self, notices: &[Notice]) -> Result<StorageMetadata> {
        let path = self.new_notices_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(notices)?;
        std::fs::write(&path, json)?;
        Ok(StorageMetadata::new(
            notices.len(),
            path.display().to_string(),
        ))
    }

    async fn rotate_to_archive(&self) -> Result<StorageMetadata> {
        let new_path = self.new_notices_path();
        if !new_path.exists() {
            return Ok(StorageMetadata::new(0, "No notices to rotate"));
        }

        let notices = self.load_new().await?;
        let now = Utc::now();
        let archive_path = self.archive_path(now.year(), now.month());

        if let Some(parent) = archive_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&notices)?;
        std::fs::write(&archive_path, json)?;

        // Remove the "New" file after archiving
        std::fs::remove_file(&new_path)?;

        Ok(StorageMetadata::new(
            notices.len(),
            archive_path.display().to_string(),
        ))
    }

    async fn load_new(&self) -> Result<Vec<Notice>> {
        let path = self.new_notices_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let notices: Vec<Notice> = serde_json::from_str(&content)?;
        Ok(notices)
    }

    async fn load_archive(&self, year: i32, month: u32) -> Result<Vec<Notice>> {
        let path = self.archive_path(year, month);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let notices: Vec<Notice> = serde_json::from_str(&content)?;
        Ok(notices)
    }
}

/// S3 path utilities.
pub mod paths {
    use chrono::{DateTime, Utc};

    /// Get the S3 key for the "New" directory notices file.
    pub fn new_notices_key(bucket_prefix: &str) -> String {
        format!("{}/New/notices.json", bucket_prefix.trim_end_matches('/'))
    }

    /// Get the S3 key for monthly archive.
    pub fn monthly_archive_key(bucket_prefix: &str, timestamp: DateTime<Utc>) -> String {
        let month = timestamp.format("%Y-%m");
        format!(
            "{}/{}/notices.json",
            bucket_prefix.trim_end_matches('/'),
            month
        )
    }

    /// Get the S3 key prefix for a specific month.
    pub fn monthly_prefix(bucket_prefix: &str, year: i32, month: u32) -> String {
        format!(
            "{}/{:04}-{:02}/",
            bucket_prefix.trim_end_matches('/'),
            year,
            month
        )
    }
}

#[cfg(test)]
mod tests {
    use super::paths::*;
    use chrono::TimeZone;

    #[test]
    fn test_new_notices_key() {
        assert_eq!(new_notices_key("uRing"), "uRing/New/notices.json");
        assert_eq!(new_notices_key("uRing/"), "uRing/New/notices.json");
    }

    #[test]
    fn test_monthly_archive_key() {
        let ts = chrono::Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
        assert_eq!(
            monthly_archive_key("uRing", ts),
            "uRing/2025-01/notices.json"
        );
    }

    #[test]
    fn test_monthly_prefix() {
        assert_eq!(monthly_prefix("uRing", 2025, 1), "uRing/2025-01/");
    }
}
