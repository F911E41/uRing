// src/storage/s3.rs

//! AWS S3 storage implementation.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aws_sdk_s3::Client;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::primitives::ByteStream;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use tracing::{info, warn};

use super::paths;
use crate::error::{AppError, Result};
use crate::models::{
    Campus, CampusMeta, CategoryMeta, Config, CrawlOutcome, CrawlOutcomeReport, CrawlStats, Diff,
    LocaleConfig, Notice, NoticeCategory, NoticeIndexItem, Seed,
};
use crate::storage::{ByteReader, NoticeStorage, SnapshotMetadata, SnapshotPointer};

const DEFAULT_UPLOAD_CONCURRENCY: usize = 32;
const DEFAULT_MAX_RETRIES: usize = 3;
const DEFAULT_RETRY_BASE_DELAY_MS: u64 = 200;

// Reduced max-age for latest pointer to minimize consistency window
const CACHE_CONTROL_LATEST: &str = "public, max-age=10, stale-while-revalidate=300";
const CACHE_CONTROL_SNAPSHOT: &str = "public, max-age=31536000, immutable";
const CACHE_CONTROL_AUX: &str = "public, max-age=300, stale-while-revalidate=600";

#[derive(Clone, Copy, Debug)]
struct ObjectMetadata {
    content_type: &'static str,
    content_encoding: Option<&'static str>,
    cache_control: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct ManifestEntry {
    key: String,
    bytes: u64,
    sha256: String,
    content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SnapshotManifest {
    schema_version: u32,
    version: String,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    entries: Vec<ManifestEntry>,
}

/// S3-based notice storage.
#[derive(Clone)]
pub struct S3Storage {
    client: Client,
    bucket: String,
    prefix: String,
    upload_concurrency: usize,
    max_retries: usize,
    retry_base_delay: Duration,
}

#[async_trait]
impl ByteReader for S3Storage {
    async fn read_bytes_optional(&self, key: &str) -> Result<Option<Vec<u8>>> {
        S3Storage::read_bytes_optional(self, key).await
    }
}

impl S3Storage {
    /// Create a new S3 storage instance.
    pub fn new(client: Client, bucket: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            client,
            bucket: bucket.into(),
            prefix: prefix.into(),
            upload_concurrency: DEFAULT_UPLOAD_CONCURRENCY,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_base_delay: Duration::from_millis(DEFAULT_RETRY_BASE_DELAY_MS),
        }
    }

    /// Create S3 storage from environment configuration.
    pub async fn from_env() -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);

        let bucket =
            std::env::var("S3_BUCKET").map_err(|_| AppError::config("S3_BUCKET not set"))?;
        let prefix = std::env::var("S3_PREFIX").unwrap_or_default();

        let mut storage = Self::new(client, bucket, prefix);
        if let Ok(value) = std::env::var("S3_UPLOAD_CONCURRENCY") {
            if let Ok(parsed) = value.parse::<usize>() {
                storage.upload_concurrency = parsed.max(1);
            }
        }
        if let Ok(value) = std::env::var("S3_MAX_RETRIES") {
            if let Ok(parsed) = value.parse::<usize>() {
                storage.max_retries = parsed;
            }
        }
        if let Ok(value) = std::env::var("S3_RETRY_BASE_DELAY_MS") {
            if let Ok(parsed) = value.parse::<u64>() {
                storage.retry_base_delay = Duration::from_millis(parsed);
            }
        }

        Ok(storage)
    }

    /// Helper to resolve logical key to absolute S3 key including prefix.
    /// This enforces Solution A: Internal prefix application.
    fn resolve_key(&self, logical_key: &str) -> String {
        if self.prefix.is_empty() {
            logical_key.to_string()
        } else {
            // Ensure we don't double-slash. logical_key usually comes from paths::* which might be relative or absolute path style.
            format!(
                "{}/{}",
                self.prefix.trim_end_matches('/'),
                logical_key.trim_start_matches('/')
            )
        }
    }

    /// Read raw bytes from S3, returning None if the key does not exist.
    pub async fn read_bytes_optional(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let final_key = self.resolve_key(key);
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&final_key)
            .send()
            .await;

        match result {
            Ok(output) => {
                let bytes = output.body.collect().await.map_err(|e| {
                    AppError::S3(format!(
                        "Failed to collect object body for key {}: {}",
                        final_key, e
                    ))
                })?;
                Ok(Some(bytes.into_bytes().to_vec()))
            }
            Err(err) => {
                if let SdkError::ServiceError(service_err) = &err {
                    if service_err.err().is_no_such_key() {
                        info!("No existing data at s3://{}/{}", self.bucket, final_key);
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

    fn json_metadata(cache_control: Option<&'static str>) -> ObjectMetadata {
        ObjectMetadata {
            content_type: "application/json; charset=utf-8",
            content_encoding: None,
            cache_control,
        }
    }

    fn text_metadata(cache_control: Option<&'static str>) -> ObjectMetadata {
        ObjectMetadata {
            content_type: "text/plain; charset=utf-8",
            content_encoding: None,
            cache_control,
        }
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    fn snapshot_version(start_time: DateTime<Utc>) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{}-{:x}", start_time.format("%Y%m%d%H%M%S"), nanos)
    }

    fn retry_delay(&self, attempt: usize) -> Duration {
        let base_ms = self.retry_base_delay.as_millis() as u64;
        let factor = 2_u64.saturating_pow(attempt.min(6) as u32);
        Duration::from_millis(base_ms.saturating_mul(factor))
    }

    /// Check if the error is transient and should be retried.
    fn should_retry(err: &SdkError<aws_sdk_s3::operation::put_object::PutObjectError>) -> bool {
        match err {
            SdkError::ServiceError(e) => {
                let status = e.raw().status().as_u16();
                // Retry on 429 (Too Many Requests) or 5xx (Server Errors)
                status == 429 || (500..=599).contains(&status)
            }
            SdkError::TimeoutError(_) | SdkError::DispatchFailure(_) => true,
            _ => false, // 403, 404, 400, etc. are fatal
        }
    }

    // Use `Bytes` for cheap cloning.
    async fn put_bytes_with_retry(
        &self,
        key: &str,
        bytes: Bytes,
        metadata: &ObjectMetadata,
    ) -> Result<()> {
        let final_key = self.resolve_key(key);
        let mut retries = 0;

        loop {
            // Bytes::clone() is cheap (Arc internally)
            let body = ByteStream::from(bytes.clone());
            let mut request = self
                .client
                .put_object()
                .bucket(&self.bucket)
                .key(&final_key)
                .body(body)
                .content_type(metadata.content_type);

            if let Some(encoding) = metadata.content_encoding {
                request = request.content_encoding(encoding);
            }
            if let Some(cache_control) = metadata.cache_control {
                request = request.cache_control(cache_control);
            }

            match request.send().await {
                Ok(_) => {
                    info!("Wrote object to s3://{}/{}", self.bucket, final_key);
                    return Ok(());
                }
                Err(err) => {
                    // Smart retry logic
                    if retries >= self.max_retries || !Self::should_retry(&err) {
                        return Err(AppError::S3(err.to_string()));
                    }
                    retries += 1;
                    let delay = self.retry_delay(retries);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    async fn write_json_entry<T: Serialize + ?Sized>(
        &self,
        key: &str,
        value: &T,
        metadata: ObjectMetadata,
    ) -> Result<ManifestEntry> {
        let vec_bytes = serde_json::to_vec(value)?;
        let len = vec_bytes.len() as u64;
        let sha256 = Self::sha256_hex(&vec_bytes);

        let bytes = Bytes::from(vec_bytes); // Convert to Bytes

        let entry = ManifestEntry {
            key: key.to_string(),
            bytes: len,
            sha256,
            content_type: metadata.content_type.to_string(),
            content_encoding: metadata.content_encoding.map(|v| v.to_string()),
            cache_control: metadata.cache_control.map(|v| v.to_string()),
        };
        self.put_bytes_with_retry(key, bytes, &metadata).await?;
        Ok(entry)
    }

    async fn write_bytes_entry(
        &self,
        key: &str,
        bytes: Vec<u8>,
        metadata: ObjectMetadata,
    ) -> Result<ManifestEntry> {
        let len = bytes.len() as u64;
        let sha256 = Self::sha256_hex(&bytes);

        let bytes = Bytes::from(bytes); // Convert to Bytes

        let entry = ManifestEntry {
            key: key.to_string(),
            bytes: len,
            sha256,
            content_type: metadata.content_type.to_string(),
            content_encoding: metadata.content_encoding.map(|v| v.to_string()),
            cache_control: metadata.cache_control.map(|v| v.to_string()),
        };
        self.put_bytes_with_retry(key, bytes, &metadata).await?;
        Ok(entry)
    }

    async fn write_json<T: Serialize + ?Sized>(
        &self,
        key: &str,
        value: &T,
        metadata: ObjectMetadata,
    ) -> Result<()> {
        let _ = self.write_json_entry(key, value, metadata).await?;
        Ok(())
    }

    async fn write_bytes(&self, key: &str, bytes: Vec<u8>, metadata: ObjectMetadata) -> Result<()> {
        let _ = self.write_bytes_entry(key, bytes, metadata).await?;
        Ok(())
    }

    async fn write_marker(&self, key: &str) -> Result<()> {
        let metadata = Self::text_metadata(Some(CACHE_CONTROL_SNAPSHOT));
        self.put_bytes_with_retry(key, Bytes::new(), &metadata)
            .await
    }

    async fn object_exists(&self, key: &str) -> Result<bool> {
        let final_key = self.resolve_key(key);
        let result = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&final_key)
            .send()
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(err) => {
                if let SdkError::ServiceError(service_err) = &err {
                    if service_err.err().is_not_found() {
                        return Ok(false);
                    }
                }
                Err(AppError::S3(err.to_string()))
            }
        }
    }

    async fn delete_object_if_exists(&self, key: &str) -> Result<()> {
        let final_key = self.resolve_key(key);
        let result = self
            .client
            .delete_object()
            .bucket(&self.bucket)
            .key(&final_key)
            .send()
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(err) => {
                warn!(
                    "Failed to delete s3://{}/{}: {}",
                    self.bucket, final_key, err
                );
                Ok(())
            }
        }
    }
}

#[async_trait]
impl NoticeStorage for S3Storage {
    async fn write_config_bundle(
        &self,
        config: &Config,
        seed: &Seed,
        locale: &LocaleConfig,
        site_map: &[Campus],
    ) -> Result<()> {
        let config_key = paths::config_key(&self.prefix, "config.toml");
        let seed_key = paths::config_key(&self.prefix, "seed.toml");
        let locale_key = paths::config_key(&self.prefix, "locale.toml");
        let site_map_key = paths::config_key(&self.prefix, "siteMap.json");

        let config_toml = toml::to_string(config)?;
        let seed_toml = toml::to_string(seed)?;
        let locale_toml = toml::to_string(locale)?;

        let text_meta = Self::text_metadata(Some(CACHE_CONTROL_SNAPSHOT));
        let json_meta = Self::json_metadata(Some(CACHE_CONTROL_SNAPSHOT));

        self.write_bytes(&config_key, config_toml.into_bytes(), text_meta)
            .await?;
        self.write_bytes(&seed_key, seed_toml.into_bytes(), text_meta)
            .await?;
        self.write_bytes(&locale_key, locale_toml.into_bytes(), text_meta)
            .await?;
        self.write_json(&site_map_key, site_map, json_meta).await?;

        Ok(())
    }

    async fn write_snapshot(
        &self,
        outcome: &CrawlOutcome,
        campuses: &[Campus],
        stats: &CrawlStats,
    ) -> Result<SnapshotMetadata> {
        let notices: &[Notice] = &outcome.notices;
        let start_time = Utc::now();
        let version = Self::snapshot_version(start_time);

        // Solution A: Pass empty string for prefix here too
        let snapshot_prefix = paths::snapshot_prefix("", &version);
        let in_progress_key = paths::in_progress_key(&snapshot_prefix);
        self.write_marker(&in_progress_key).await?;

        // 1. Load previous snapshot for diffing
        let pointer_key = paths::pointer_key("");
        let previous_pointer = self
            .read_json_optional::<SnapshotPointer>(&pointer_key)
            .await?;
        let previous_items: Vec<NoticeIndexItem> = if let Some(pointer) = &previous_pointer {
            // Note: pointer.version is just the version string, so this is safe
            let old_snapshot_prefix = paths::snapshot_prefix("", &pointer.version);
            let old_index_key = paths::index_key(&old_snapshot_prefix, "all.json");
            self.read_json_optional(&old_index_key)
                .await?
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Compute diff between current and previous snapshots
        let previous_items_map: HashMap<String, Option<String>> = previous_items
            .into_iter()
            .map(|item| (item.id, item.content_hash))
            .collect();

        let mut manifest_entries: Vec<ManifestEntry> = Vec::new();

        // 2. Write detail files in parallel
        let detail_meta = Self::json_metadata(Some(CACHE_CONTROL_SNAPSHOT));
        let detail_entries = stream::iter(notices.iter().cloned())
            .map(|notice| {
                let key = paths::detail_key(&snapshot_prefix, &notice.canonical_id());
                // write_json_entry now handles auto-prefixing via resolve_key
                async move { self.write_json_entry(&key, &notice, detail_meta).await }
            })
            .buffer_unordered(self.upload_concurrency)
            .collect::<Vec<_>>()
            .await;

        for entry in detail_entries {
            manifest_entries.push(entry?);
        }

        // 3. Create and write index files
        let index_items: Vec<NoticeIndexItem> = notices.iter().map(NoticeIndexItem::from).collect();
        let all_index_key = paths::index_key(&snapshot_prefix, "all.json");
        let index_meta = Self::json_metadata(Some(CACHE_CONTROL_SNAPSHOT));
        manifest_entries.push(
            self.write_json_entry(&all_index_key, &index_items, index_meta)
                .await?,
        );

        // Compute diff between current and previous snapshots
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
            manifest_entries.push(self.write_json_entry(&key, items, index_meta).await?);
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
            manifest_entries.push(self.write_json_entry(&key, items, index_meta).await?);
        }

        // 4. Meta files
        let campus_meta: Vec<CampusMeta> = campuses.iter().map(CampusMeta::from).collect();
        let meta_key = paths::meta_key(&snapshot_prefix, "campus.json");
        manifest_entries.push(
            self.write_json_entry(&meta_key, &campus_meta, index_meta)
                .await?,
        );
        let category_key = paths::meta_key(&snapshot_prefix, "category.json");
        manifest_entries.push(
            self.write_json_entry(&category_key, &CategoryMeta::all(), index_meta)
                .await?,
        );
        let source_key = paths::meta_key(&snapshot_prefix, "source.json");
        manifest_entries.push(
            self.write_json_entry(&source_key, campuses, index_meta)
                .await?,
        );

        // 5. Aux files
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
        let mut removed = Vec::new();

        // Compute difference between current and previous snapshots
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
        for id in previous_items_map.keys() {
            if !current_ids.contains(id) {
                removed.push(id.clone());
            }
        }

        added.sort();
        updated.sort();
        removed.sort();
        let diff = Diff {
            added,
            updated,
            removed,
        };
        let aux_meta = Self::json_metadata(Some(CACHE_CONTROL_AUX));
        let diff_key = paths::aux_key(&snapshot_prefix, "diff.json");
        manifest_entries.push(self.write_json_entry(&diff_key, &diff, aux_meta).await?);

        let stats_key = paths::aux_key(&snapshot_prefix, "stats.json");
        manifest_entries.push(self.write_json_entry(&stats_key, stats, aux_meta).await?);

        let outcome_report = CrawlOutcomeReport::from(outcome);
        let outcome_key = paths::aux_key(&snapshot_prefix, "outcome.json");
        manifest_entries.push(
            self.write_json_entry(&outcome_key, &outcome_report, aux_meta)
                .await?,
        );
        if !outcome.errors.is_empty() {
            let error_key = paths::aux_key(&snapshot_prefix, "errors.json");
            manifest_entries.push(
                self.write_json_entry(&error_key, &outcome.errors, aux_meta)
                    .await?,
            );
        }

        // 6. Write manifest and commit marker
        manifest_entries.sort_by(|a, b| a.key.cmp(&b.key));
        let manifest = SnapshotManifest {
            schema_version: 1,
            version: version.clone(),
            started_at: start_time,
            finished_at: Utc::now(),
            entries: manifest_entries,
        };
        let manifest_key = paths::manifest_key(&snapshot_prefix);
        let _ = self
            .write_json_entry(&manifest_key, &manifest, index_meta)
            .await?;
        let success_key = paths::success_key(&snapshot_prefix);
        self.write_marker(&success_key).await?;
        self.delete_object_if_exists(&in_progress_key).await?;

        // 7. Atomically update pointer
        let pointer = SnapshotPointer::new(version.clone());
        if let Some(previous) = previous_pointer {
            let previous_key = paths::previous_pointer_key("");
            self.write_json(&previous_key, &previous, aux_meta).await?;
        }
        self.write_json(
            &pointer_key,
            &pointer,
            Self::json_metadata(Some(CACHE_CONTROL_LATEST)),
        )
        .await?;

        Ok(SnapshotMetadata {
            notice_count: notices.len(),
            timestamp: start_time,
            snapshot_location: format!(
                "s3://{}/{}",
                self.bucket,
                self.resolve_key(&snapshot_prefix)
            ),
            pointer_location: format!("s3://{}/{}", self.bucket, self.resolve_key(&pointer_key)),
        })
    }

    async fn load_snapshot(&self) -> Result<Vec<NoticeIndexItem>> {
        let pointer_key = paths::pointer_key("");
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

        let snapshot_prefix = paths::snapshot_prefix("", &pointer.version);
        let success_key = paths::success_key(&snapshot_prefix);
        if !self.object_exists(&success_key).await? {
            warn!("latest.json points to an incomplete snapshot.");
            return Ok(Vec::new());
        }

        let index_key = paths::index_key(&snapshot_prefix, "all.json");
        Ok(self
            .read_json_optional(&index_key)
            .await?
            .unwrap_or_default())
    }
}
