// src/storage/mod.rs

//! Storage abstractions for notice persistence.
//!
//! This module provides a unified interface for storing notices,
//! with implementations for local filesystem and AWS S3.

// #[cfg(feature = "lambda")]
pub mod s3;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::{Campus, Config, CrawlStats, Notice, NoticeIndexItem, Seed};

/// Metadata about a snapshot operation.
#[derive(Debug, Clone)]
pub struct SnapshotMetadata {
    /// Number of notices stored in the snapshot
    pub notice_count: usize,
    /// Timestamp of the operation
    pub timestamp: DateTime<Utc>,
    /// Snapshot location (path or S3 key)
    pub snapshot_location: String,
    /// Pointer location (path or S3 key)
    pub pointer_location: String,
}

/// Pointer file contents for latest snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotPointer {
    pub version: String,
    pub updated_at: DateTime<Utc>,
}

impl SnapshotPointer {
    pub fn new(version: String) -> Self {
        Self {
            version,
            updated_at: Utc::now(),
        }
    }
}

/// Trait for notice storage backends.
#[async_trait]
pub trait NoticeStorage: Send + Sync {
    /// Write config, seed, and site map to storage.
    async fn write_config_bundle(
        &self,
        config: &Config,
        seed: &Seed,
        site_map: &[Campus],
    ) -> Result<()>;

    /// Write a snapshot of new notices and update the pointer.
    async fn write_snapshot(
        &self,
        notices: &[Notice],
        campuses: &[Campus],
        stats: &CrawlStats,
    ) -> Result<SnapshotMetadata>;

    /// Load notices from the latest snapshot pointer.
    async fn load_snapshot(&self) -> Result<Vec<NoticeIndexItem>>;
}

/// S3 path utilities.
pub mod paths {
    use crate::models::NoticeCategory;

    fn join(base: &str, path: &str) -> String {
        let base = base.trim_matches('/');
        let path = path.trim_start_matches('/');
        if base.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", base, path)
        }
    }

    /// Get the S3 key for a config file at the root.
    pub fn config_key(bucket_prefix: &str, file_name: &str) -> String {
        join(bucket_prefix, &format!("config/{}", file_name))
    }

    /// Get the S3 key for the pointer file.
    pub fn pointer_key(bucket_prefix: &str) -> String {
        join(bucket_prefix, "latest.json")
    }

    /// Get the S3 prefix for a specific snapshot version.
    pub fn snapshot_prefix(bucket_prefix: &str, version: &str) -> String {
        join(bucket_prefix, &format!("snapshots/{}", version))
    }

    /// Get the S3 key for an index file within a snapshot.
    pub fn index_key(snapshot_prefix: &str, file_name: &str) -> String {
        join(snapshot_prefix, &format!("index/{}", file_name))
    }

    /// Get the S3 key for a campus-specific index file.
    pub fn campus_index_key(snapshot_prefix: &str, campus_id: &str) -> String {
        join(snapshot_prefix, &format!("index/campus/{}.json", campus_id))
    }

    /// Get the S3 key for a category-specific index file.
    pub fn category_index_key(snapshot_prefix: &str, category: &NoticeCategory) -> String {
        join(
            snapshot_prefix,
            &format!("index/category/{:?}.json", category).to_lowercase(),
        )
    }

    /// Get the S3 key for a meta file within a snapshot.
    pub fn meta_key(snapshot_prefix: &str, file_name: &str) -> String {
        join(snapshot_prefix, &format!("meta/{}", file_name))
    }

    /// Get the S3 key for a detail file within a snapshot.
    pub fn detail_key(snapshot_prefix: &str, notice_id: &str) -> String {
        join(snapshot_prefix, &format!("detail/{}.json", notice_id))
    }

    /// Get the S3 key for an auxiliary file within a snapshot.
    pub fn aux_key(snapshot_prefix: &str, file_name: &str) -> String {
        join(snapshot_prefix, &format!("aux/{}", file_name))
    }
}

#[cfg(test)]
mod tests {
    use super::paths::*;
    use crate::models::NoticeCategory;

    const PREFIX: &str = "uRing";
    const VERSION: &str = "20250101120000";

    #[test]
    fn test_pointer_key() {
        assert_eq!(pointer_key(PREFIX), "uRing/latest.json");
        assert_eq!(pointer_key("uRing/"), "uRing/latest.json");
        assert_eq!(pointer_key(""), "latest.json");
    }

    #[test]
    fn test_config_key() {
        assert_eq!(
            config_key(PREFIX, "config.toml"),
            "uRing/config/config.toml"
        );
        assert_eq!(config_key("", "config.toml"), "config/config.toml");
    }

    #[test]
    fn test_snapshot_prefix() {
        assert_eq!(
            snapshot_prefix(PREFIX, VERSION),
            "uRing/snapshots/20250101120000"
        );
    }

    #[test]
    fn test_index_key() {
        let prefix = snapshot_prefix(PREFIX, VERSION);
        assert_eq!(
            index_key(&prefix, "all.json"),
            "uRing/snapshots/20250101120000/index/all.json"
        );
    }

    #[test]
    fn test_campus_index_key() {
        let prefix = snapshot_prefix(PREFIX, VERSION);
        assert_eq!(
            campus_index_key(&prefix, "seoul"),
            "uRing/snapshots/20250101120000/index/campus/seoul.json"
        );
    }

    #[test]
    fn test_category_index_key() {
        let prefix = snapshot_prefix(PREFIX, VERSION);
        assert_eq!(
            category_index_key(&prefix, &NoticeCategory::Academic),
            "uRing/snapshots/20250101120000/index/category/academic.json"
        );
    }

    #[test]
    fn test_meta_key() {
        let prefix = snapshot_prefix(PREFIX, VERSION);
        assert_eq!(
            meta_key(&prefix, "campus.json"),
            "uRing/snapshots/20250101120000/meta/campus.json"
        );
    }

    #[test]
    fn test_detail_key() {
        let prefix = snapshot_prefix(PREFIX, VERSION);
        assert_eq!(
            detail_key(&prefix, "abcdef123"),
            "uRing/snapshots/20250101120000/detail/abcdef123.json"
        );
    }

    #[test]
    fn test_aux_key() {
        let prefix = snapshot_prefix(PREFIX, VERSION);
        assert_eq!(
            aux_key(&prefix, "diff.json"),
            "uRing/snapshots/20250101120000/aux/diff.json"
        );
    }
}
