// src/storage/s3.rs

//! AWS S3 storage implementation.

use std::collections::{HashMap, HashSet};

use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;

use async_trait::async_trait;
use chrono::Utc;
use futures::future::join_all;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::{
    Campus, CampusMeta, CategoryMeta, Config, CrawlStats, Diff, Notice, NoticeCategory,
    NoticeIndexItem, Seed,
};
use crate::storage::{NoticeStorage, SnapshotMetadata, SnapshotPointer};

use super::paths;

/// S3-based notice storage.
#[derive(Clone)]
pub struct S3Storage {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3Storage {
    /// Create a new S3 storage instance.
    pub fn new(client: Client, bucket: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            client,
            bucket: bucket.into(),
            prefix: prefix.into(),
        }
    }

    /// Create S3 storage from environment configuration.
    pub async fn from_env() -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);

        let bucket =
            std::env::var("S3_BUCKET").map_err(|_| AppError::config("S3_BUCKET not set"))?;
        let prefix = std::env::var("S3_PREFIX").unwrap_or_default();

        Ok(Self::new(client, bucket, prefix))
    }

    /// Read raw bytes from S3, returning None if the key does not exist.
    pub async fn read_bytes_optional(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        match result {
            Ok(output) => {
                let bytes = output.body.collect().await.map_err(|e| {
                    AppError::S3(format!(
                        "Failed to collect object body for key {}: {}",
                        key, e
                    ))
                })?;
                Ok(Some(bytes.into_bytes().to_vec()))
            }
            Err(err) => {
                if let aws_sdk_s3::error::SdkError::ServiceError(service_err) = &err {
                    if service_err.err().is_no_such_key() {
                        info!("No existing data at s3://{}/{}", self.bucket, key);
                        return Ok(None);
                    }
                }
                Err(AppError::S3(err.to_string()))
            }
        }
    }

    /// Read JSON from S3, returning None if the key does not exist.
    pub async fn read_json_optional<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.read_bytes_optional(key).await? {
            Some(bytes) => {
                let value: T = serde_json::from_slice(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Write a serializable object as JSON to S3.
    async fn write_json<T: Serialize + ?Sized>(&self, key: &str, value: &T) -> Result<()> {
        let bytes = serde_json::to_vec(value)?;
        let body = ByteStream::from(bytes);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;

        info!("Wrote object to s3://{}/{}", self.bucket, key);
        Ok(())
    }

    /// Write raw bytes to S3 with a content type.
    async fn write_bytes(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<()> {
        let body = ByteStream::from(bytes);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;

        info!("Wrote object to s3://{}/{}", self.bucket, key);
        Ok(())
    }
}

#[async_trait]
impl NoticeStorage for S3Storage {
    async fn write_config_bundle(
        &self,
        config: &Config,
        seed: &Seed,
        site_map: &[Campus],
    ) -> Result<()> {
        let config_key = paths::config_key(&self.prefix, "config.toml");
        let seed_key = paths::config_key(&self.prefix, "seed.toml");
        let site_map_key = paths::config_key(&self.prefix, "siteMap.json");

        let config_toml = toml::to_string(config)?;
        let seed_toml = toml::to_string(seed)?;

        self.write_bytes(&config_key, config_toml.into_bytes(), "text/plain")
            .await?;
        self.write_bytes(&seed_key, seed_toml.into_bytes(), "text/plain")
            .await?;
        self.write_json(&site_map_key, site_map).await?;

        Ok(())
    }

    async fn write_snapshot(
        &self,
        notices: &[Notice],
        campuses: &[Campus],
        stats: &CrawlStats,
    ) -> Result<SnapshotMetadata> {
        let start_time = Utc::now();
        let version = start_time.format("%Y%m%d%H%M%S").to_string();
        let snapshot_prefix = paths::snapshot_prefix(&self.prefix, &version);

        // 1. Load previous snapshot for diffing
        let pointer_key = paths::pointer_key(&self.prefix);
        let previous_items: Vec<NoticeIndexItem> = if let Some(pointer) = self
            .read_json_optional::<SnapshotPointer>(&pointer_key)
            .await?
        {
            let old_snapshot_prefix = paths::snapshot_prefix(&self.prefix, &pointer.version);
            let old_index_key = paths::index_key(&old_snapshot_prefix, "all.json");
            self.read_json_optional(&old_index_key)
                .await?
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let previous_items_map: HashMap<String, Option<String>> = previous_items
            .into_iter()
            .map(|item| (item.id, item.content_hash))
            .collect();

        // 2. Write detail files in parallel
        let uploads_data: Vec<_> = notices
            .iter()
            .map(|notice| {
                let notice_id = notice.canonical_id();
                let key = paths::detail_key(&snapshot_prefix, &notice_id);
                (key, notice)
            })
            .collect();

        let detail_uploads = uploads_data
            .iter()
            .map(|(key, notice)| self.write_json(key, *notice));

        join_all(detail_uploads)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        // 3. Create and write index files
        let index_items: Vec<NoticeIndexItem> = notices.iter().map(NoticeIndexItem::from).collect();
        let all_index_key = paths::index_key(&snapshot_prefix, "all.json");
        self.write_json(&all_index_key, &index_items).await?;

        // Per-campus indices
        let notice_by_id: HashMap<String, &Notice> = notices
            .iter()
            .map(|notice| (notice.canonical_id(), notice))
            .collect();
        let mut campus_map: HashMap<String, Vec<&NoticeIndexItem>> = HashMap::new();
        for item in &index_items {
            if let Some(notice) = notice_by_id.get(&item.id) {
                campus_map
                    .entry(notice.campus.clone())
                    .or_default()
                    .push(item);
            }
        }
        for (campus_id, items) in &campus_map {
            let key = paths::campus_index_key(&snapshot_prefix, campus_id);
            self.write_json(&key, items).await?;
        }

        // Per-category indices
        let mut category_map: HashMap<NoticeCategory, Vec<&NoticeIndexItem>> = HashMap::new();
        for item in &index_items {
            category_map
                .entry(item.category.clone())
                .or_default()
                .push(item);
        }
        for (category, items) in &category_map {
            let key = paths::category_index_key(&snapshot_prefix, category);
            self.write_json(&key, items).await?;
        }

        // 4. Create and write meta files
        let campus_meta: Vec<CampusMeta> = campuses.iter().map(CampusMeta::from).collect();
        self.write_json(
            &paths::meta_key(&snapshot_prefix, "campus.json"),
            &campus_meta,
        )
        .await?;
        self.write_json(
            &paths::meta_key(&snapshot_prefix, "category.json"),
            &CategoryMeta::all(),
        )
        .await?;
        self.write_json(&paths::meta_key(&snapshot_prefix, "source.json"), campuses)
            .await?;

        // 5. Create and write aux files
        let current_ids: HashSet<String> = index_items.iter().map(|item| item.id.clone()).collect();
        let current_hashes: HashMap<String, String> = index_items
            .iter()
            .filter_map(|item| {
                item.content_hash
                    .clone()
                    .map(|hash| (item.id.clone(), hash))
            })
            .collect();
        let mut added = Vec::new();
        let mut updated = Vec::new();

        for id in &current_ids {
            match previous_items_map.get(id) {
                None => added.push(id.clone()),
                Some(Some(prev_hash)) => {
                    if let Some(current_hash) = current_hashes.get(id) {
                        if current_hash != prev_hash {
                            updated.push(id.clone());
                        }
                    }
                }
                Some(None) => {}
            }
        }

        added.sort();
        updated.sort();
        let diff = Diff { added, updated };
        self.write_json(&paths::aux_key(&snapshot_prefix, "diff.json"), &diff)
            .await?;
        self.write_json(&paths::aux_key(&snapshot_prefix, "stats.json"), stats)
            .await?;

        // 6. Atomically update pointer
        let pointer = SnapshotPointer::new(version.clone());
        self.write_json(&pointer_key, &pointer).await?;

        Ok(SnapshotMetadata {
            notice_count: notices.len(),
            timestamp: start_time,
            snapshot_location: format!("s3://{}/{}", self.bucket, snapshot_prefix),
            pointer_location: format!("s3://{}/{}", self.bucket, pointer_key),
        })
    }

    async fn load_snapshot(&self) -> Result<Vec<NoticeIndexItem>> {
        let pointer_key = paths::pointer_key(&self.prefix);
        let pointer = match self
            .read_json_optional::<SnapshotPointer>(&pointer_key)
            .await?
        {
            Some(p) => p,
            None => {
                warn!("latest.json pointer not found.");
                return Ok(Vec::new());
            }
        };

        let index_key = paths::index_key(
            &paths::snapshot_prefix(&self.prefix, &pointer.version),
            "all.json",
        );
        Ok(self
            .read_json_optional(&index_key)
            .await?
            .unwrap_or_default())
    }
}
