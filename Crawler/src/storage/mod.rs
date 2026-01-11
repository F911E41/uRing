//! Storage abstractions for notice persistence.
//!
//! This module provides a unified interface for storing notices,
//! with implementations for local filesystem and AWS S3.

#[cfg(feature = "lambda")]
pub mod s3;

use std::future::Future;

use chrono::{DateTime, Utc};

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

/// Trait for notice storage backends.
pub trait NoticeStorage {
    /// Store notices in the "New" directory (hot data for notifications).
    fn store_new(
        &self,
        notices: &[Notice],
    ) -> impl Future<Output = Result<StorageMetadata>> + Send;

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
