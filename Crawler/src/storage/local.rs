// src/storage/local.rs

//! Local filesystem storage implementation.
//! Mirrors the S3 key-space by mapping keys -> files under root_dir.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::{
    Campus, CampusMeta, CategoryMeta, Config, CrawlOutcome, CrawlOutcomeReport, CrawlStats, Diff,
    LocaleConfig, Notice, NoticeCategory, NoticeIndexItem, Seed,
};
use crate::storage::{ByteReader, NoticeStorage, SnapshotMetadata, SnapshotPointer, paths};

const DEFAULT_WRITE_CONCURRENCY: usize = 16;

#[derive(Clone, Copy, Debug)]
struct ObjectMetadata {
    #[allow(dead_code)]
    content_type: &'static str,
    #[allow(dead_code)]
    content_encoding: Option<&'static str>,
    #[allow(dead_code)]
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

#[derive(Clone)]
pub struct LocalStorage {
    root_dir: PathBuf,
    prefix: String,
    write_concurrency: usize,
}

impl LocalStorage {
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        let prefix = std::env::var("S3_PREFIX").unwrap_or_else(|_| "uRing".to_string());
        Self::new_with_prefix(root_dir, prefix)
    }

    pub fn new_with_prefix(root_dir: impl Into<PathBuf>, prefix: impl Into<String>) -> Self {
        Self {
            root_dir: root_dir.into(),
            prefix: prefix.into(),
            write_concurrency: DEFAULT_WRITE_CONCURRENCY,
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

    fn prefix(&self) -> &str {
        self.prefix.as_str()
    }

    fn path_for_key(&self, key: &str) -> PathBuf {
        self.root_dir.join(key)
    }

    async fn ensure_parent_dir(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    async fn write_atomic(&self, key: &str, bytes: &[u8]) -> Result<()> {
        let path = self.path_for_key(key);
        Self::ensure_parent_dir(&path).await?;

        let tmp = path.with_extension("tmp");

        {
            let mut f = tokio::fs::File::create(&tmp).await?;
            f.write_all(bytes).await?;
            f.flush().await?;
        }

        if path.exists() {
            let _ = tokio::fs::remove_file(&path).await;
        }
        tokio::fs::rename(&tmp, &path).await?;
        Ok(())
    }

    async fn write_marker(&self, key: &str) -> Result<()> {
        self.write_atomic(key, &[]).await
    }

    async fn object_exists(&self, key: &str) -> Result<bool> {
        Ok(self.path_for_key(key).exists())
    }

    async fn read_json_optional<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.read_bytes_optional(key).await? {
            Some(bytes) => Ok(Some(serde_json::from_slice::<T>(&bytes)?)),
            None => Ok(None),
        }
    }

    async fn write_bytes_entry(
        &self,
        key: &str,
        bytes: Vec<u8>,
        metadata: ObjectMetadata,
    ) -> Result<ManifestEntry> {
        let entry = ManifestEntry {
            key: key.to_string(),
            bytes: bytes.len() as u64,
            sha256: Self::sha256_hex(&bytes),
            content_type: metadata.content_type.to_string(),
            content_encoding: metadata.content_encoding.map(|v| v.to_string()),
            cache_control: metadata.cache_control.map(|v| v.to_string()),
        };
        self.write_atomic(key, &bytes).await?;
        Ok(entry)
    }

    async fn write_json_entry<T: Serialize + ?Sized>(
        &self,
        key: &str,
        value: &T,
        metadata: ObjectMetadata,
    ) -> Result<ManifestEntry> {
        let bytes = serde_json::to_vec(value)?;
        self.write_bytes_entry(key, bytes, metadata).await
    }
}

#[async_trait]
impl ByteReader for LocalStorage {
    async fn read_bytes_optional(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let path = self.path_for_key(key);
        match tokio::fs::read(&path).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(AppError::LocalStorage(format!(
                "Failed to read local object {}: {}",
                path.display(),
                err
            ))),
        }
    }
}

#[async_trait]
impl NoticeStorage for LocalStorage {
    async fn write_config_bundle(
        &self,
        config: &Config,
        seed: &Seed,
        locale: &LocaleConfig,
        site_map: &[Campus],
    ) -> Result<()> {
        let prefix = self.prefix();
        let config_key = paths::config_key(prefix, "config.toml");
        let seed_key = paths::config_key(prefix, "seed.toml");
        let locale_key = paths::config_key(prefix, "locale.toml");
        let site_map_key = paths::config_key(prefix, "siteMap.json");

        let config_toml = toml::to_string(config)?;
        let seed_toml = toml::to_string(seed)?;
        let locale_toml = toml::to_string(locale)?;

        let text_meta = Self::text_metadata(None);
        let json_meta = Self::json_metadata(None);

        let _ = self
            .write_bytes_entry(&config_key, config_toml.into_bytes(), text_meta)
            .await?;
        let _ = self
            .write_bytes_entry(&seed_key, seed_toml.into_bytes(), text_meta)
            .await?;
        let _ = self
            .write_bytes_entry(&locale_key, locale_toml.into_bytes(), text_meta)
            .await?;
        let _ = self
            .write_json_entry(&site_map_key, site_map, json_meta)
            .await?;

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

        let prefix = self.prefix();
        let snapshot_prefix = paths::snapshot_prefix(prefix, &version);
        let in_progress_key = paths::in_progress_key(&snapshot_prefix);
        self.write_marker(&in_progress_key).await?;

        // previous pointer + previous index
        let pointer_key = paths::pointer_key(prefix);
        let previous_pointer = self
            .read_json_optional::<SnapshotPointer>(&pointer_key)
            .await?;

        let previous_items: Vec<NoticeIndexItem> = if let Some(pointer) = &previous_pointer {
            let old_snapshot_prefix = paths::snapshot_prefix(prefix, &pointer.version);
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

        let mut manifest_entries: Vec<ManifestEntry> = Vec::new();
        let json_meta = Self::json_metadata(None);

        // detail
        let detail_entries = stream::iter(notices.iter().cloned())
            .map(|notice| {
                let key = paths::detail_key(&snapshot_prefix, &notice.canonical_id());
                async move { self.write_json_entry(&key, &notice, json_meta).await }
            })
            .buffer_unordered(self.write_concurrency)
            .collect::<Vec<_>>()
            .await;

        for entry in detail_entries {
            manifest_entries.push(entry?);
        }

        // all index
        let index_items: Vec<NoticeIndexItem> = notices.iter().map(NoticeIndexItem::from).collect();
        let all_index_key = paths::index_key(&snapshot_prefix, "all.json");
        manifest_entries.push(
            self.write_json_entry(&all_index_key, &index_items, json_meta)
                .await?,
        );

        // per-campus
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
            manifest_entries.push(self.write_json_entry(&key, items, json_meta).await?);
        }

        // per-category
        let mut category_map: HashMap<NoticeCategory, Vec<&NoticeIndexItem>> = HashMap::new();
        for item in &index_items {
            category_map
                .entry(item.category.clone())
                .or_default()
                .push(item);
        }
        for (category, items) in &category_map {
            let key = paths::category_index_key(&snapshot_prefix, category);
            manifest_entries.push(self.write_json_entry(&key, items, json_meta).await?);
        }

        // meta
        let campus_meta: Vec<CampusMeta> = campuses.iter().map(CampusMeta::from).collect();
        let meta_key = paths::meta_key(&snapshot_prefix, "campus.json");
        manifest_entries.push(
            self.write_json_entry(&meta_key, &campus_meta, json_meta)
                .await?,
        );

        let category_key = paths::meta_key(&snapshot_prefix, "category.json");
        manifest_entries.push(
            self.write_json_entry(&category_key, &CategoryMeta::all(), json_meta)
                .await?,
        );

        let source_key = paths::meta_key(&snapshot_prefix, "source.json");
        manifest_entries.push(
            self.write_json_entry(&source_key, campuses, json_meta)
                .await?,
        );

        // aux diff/stats/outcome/errors
        let current_ids: HashSet<String> = index_items.iter().map(|item| item.id.clone()).collect();
        let current_hashes: HashMap<String, String> = index_items
            .iter()
            .filter_map(|item| item.content_hash.clone().map(|h| (item.id.clone(), h)))
            .collect();

        let mut added = Vec::new();
        let mut updated = Vec::new();
        let mut removed = Vec::new();

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
        let diff_key = paths::aux_key(&snapshot_prefix, "diff.json");
        manifest_entries.push(self.write_json_entry(&diff_key, &diff, json_meta).await?);

        let stats_key = paths::aux_key(&snapshot_prefix, "stats.json");
        manifest_entries.push(self.write_json_entry(&stats_key, stats, json_meta).await?);

        let outcome_report = CrawlOutcomeReport::from(outcome);
        let outcome_key = paths::aux_key(&snapshot_prefix, "outcome.json");
        manifest_entries.push(
            self.write_json_entry(&outcome_key, &outcome_report, json_meta)
                .await?,
        );

        if !outcome.errors.is_empty() {
            let error_key = paths::aux_key(&snapshot_prefix, "errors.json");
            manifest_entries.push(
                self.write_json_entry(&error_key, &outcome.errors, json_meta)
                    .await?,
            );
        }

        // manifest + success marker
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
            .write_json_entry(&manifest_key, &manifest, json_meta)
            .await?;

        let success_key = paths::success_key(&snapshot_prefix);
        self.write_marker(&success_key).await?;

        // remove in-progress marker (best-effort)
        let in_progress_path = self.path_for_key(&in_progress_key);
        if in_progress_path.exists() {
            let _ = tokio::fs::remove_file(in_progress_path).await;
        }

        // update pointer atomically
        let pointer = SnapshotPointer::new(version.clone());
        if let Some(previous) = previous_pointer {
            let previous_key = paths::previous_pointer_key(prefix);
            self.write_atomic(&previous_key, &serde_json::to_vec(&previous)?)
                .await?;
        }
        self.write_atomic(&pointer_key, &serde_json::to_vec(&pointer)?)
            .await?;

        info!(
            "Wrote local snapshot: {}/{}",
            self.root_dir.display(),
            snapshot_prefix
        );

        Ok(SnapshotMetadata {
            notice_count: notices.len(),
            timestamp: start_time,
            snapshot_location: format!("{}/{}", self.root_dir.display(), snapshot_prefix),
            pointer_location: format!("{}/{}", self.root_dir.display(), pointer_key),
        })
    }

    async fn load_snapshot(&self) -> Result<Vec<NoticeIndexItem>> {
        let prefix = self.prefix();
        let pointer_key = paths::pointer_key(prefix);

        let pointer = match self
            .read_json_optional::<SnapshotPointer>(&pointer_key)
            .await?
        {
            Some(p) => p,
            None => {
                warn!("latest.json pointer not found (local).");
                return Ok(Vec::new());
            }
        };

        let snapshot_prefix = paths::snapshot_prefix(prefix, &pointer.version);
        let success_key = paths::success_key(&snapshot_prefix);

        if !self.object_exists(&success_key).await? {
            warn!("latest.json points to an incomplete snapshot (local).");
            return Ok(Vec::new());
        }

        let index_key = paths::index_key(&snapshot_prefix, "all.json");
        Ok(self
            .read_json_optional(&index_key)
            .await?
            .unwrap_or_default())
    }
}
