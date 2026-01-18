// src/storage/mod.rs

//! Storage abstractions for notice persistence.
//!
//! Unified interface for storing crawler outputs with multiple backends.

// Local is default storage backend
pub mod local;

#[cfg(feature = "s3")]
pub mod s3;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::{
    Campus, Config, CrawlOutcome, CrawlStats, LocaleConfig, NoticeIndexItem, Seed,
};

/// Metadata about a snapshot operation.
#[derive(Debug, Clone)]
pub struct SnapshotMetadata {
    pub notice_count: usize,
    pub timestamp: DateTime<Utc>,
    pub snapshot_location: String,
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

/// Minimal read interface used by config loaders.
/// (So config loading does NOT depend on concrete storage types.)
#[async_trait]
pub trait ByteReader: Send + Sync {
    async fn read_bytes_optional(&self, key: &str) -> Result<Option<Vec<u8>>>;
}

/// Trait for notice storage backends.
#[async_trait]
pub trait NoticeStorage: ByteReader {
    /// Write config/seed/locale/sitemap bundle to storage.
    async fn write_config_bundle(
        &self,
        config: &Config,
        seed: &Seed,
        locale: &LocaleConfig,
        site_map: &[Campus],
    ) -> Result<()>;

    async fn write_snapshot(
        &self,
        outcome: &CrawlOutcome,
        campuses: &[Campus],
        stats: &CrawlStats,
    ) -> Result<SnapshotMetadata>;

    async fn load_snapshot(&self) -> Result<Vec<NoticeIndexItem>>;
}

/// Path utilities (logical key-space shared by all backends).
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

    pub fn config_key(bucket_prefix: &str, file_name: &str) -> String {
        join(bucket_prefix, &format!("config/{}", file_name))
    }

    pub fn pointer_key(bucket_prefix: &str) -> String {
        join(bucket_prefix, "latest.json")
    }

    pub fn previous_pointer_key(bucket_prefix: &str) -> String {
        join(bucket_prefix, "previous.json")
    }

    pub fn snapshot_prefix(bucket_prefix: &str, version: &str) -> String {
        join(bucket_prefix, &format!("snapshots/{}", version))
    }

    pub fn index_key(snapshot_prefix: &str, file_name: &str) -> String {
        join(snapshot_prefix, &format!("index/{}", file_name))
    }

    pub fn campus_index_key(snapshot_prefix: &str, campus_id: &str) -> String {
        join(snapshot_prefix, &format!("index/campus/{}.json", campus_id))
    }

    pub fn category_index_key(snapshot_prefix: &str, category: &NoticeCategory) -> String {
        join(
            snapshot_prefix,
            &format!("index/category/{:?}.json", category).to_lowercase(),
        )
    }

    pub fn meta_key(snapshot_prefix: &str, file_name: &str) -> String {
        join(snapshot_prefix, &format!("meta/{}", file_name))
    }

    pub fn manifest_key(snapshot_prefix: &str) -> String {
        join(snapshot_prefix, "_manifest.json")
    }

    pub fn success_key(snapshot_prefix: &str) -> String {
        join(snapshot_prefix, "_SUCCESS")
    }

    pub fn in_progress_key(snapshot_prefix: &str) -> String {
        join(snapshot_prefix, "_IN_PROGRESS")
    }

    pub fn detail_key(snapshot_prefix: &str, notice_id: &str) -> String {
        join(snapshot_prefix, &format!("detail/{}.json", notice_id))
    }

    pub fn aux_key(snapshot_prefix: &str, file_name: &str) -> String {
        join(snapshot_prefix, &format!("aux/{}", file_name))
    }
}
