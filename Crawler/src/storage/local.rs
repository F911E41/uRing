//! Local filesystem storage implementation.
//!
//! Implements Hot/Cold storage pattern with Circuit Breaker and Inverted Index
//! for development and testing. Production deployments should use S3Storage.
//!
//! ## Storage Layout
//!
//! ```text
//! {root}/
//! ├── config.toml           # Crawler Configuration
//! ├── index.json            # Inverted Index for Search
//! ├── current.json          # Hot: Active Window (Write-Buffer)
//! ├── siteMap.json          # Site Map for Crawling
//! └── stacks/               # Cold: Immutable Archives
//!     └── YYYY/
//!         └── MM.json
//! ```
//!
//! ## Features
//!
//! - **Circuit Breaker**: Aborts write if notice count drops >20%
//! - **Inverted Index**: Generates `index.json` for client-side search
//! - **Diff Calculation**: Returns changes for notification dispatch

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{Datelike, Utc};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, Result};
use crate::models::{Campus, CrawlOutcome, CrawlStats, NoticeOutput};
use crate::pipeline::{CircuitBreaker, InvertedIndex, build_index, calculate_diff};
use crate::storage::{CurrentData, NoticeStorage, WriteMetadata, WriteOptions};

/// Local filesystem storage backend.
#[derive(Clone)]
pub struct LocalStorage {
    root_dir: PathBuf,
    circuit_breaker: CircuitBreaker,
}

impl LocalStorage {
    /// Create a new LocalStorage rooted at the given directory.
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
            circuit_breaker: CircuitBreaker::new(),
        }
    }

    /// Create a LocalStorage with custom circuit breaker configuration.
    pub fn with_circuit_breaker(
        root_dir: impl Into<PathBuf>,
        circuit_breaker: CircuitBreaker,
    ) -> Self {
        Self {
            root_dir: root_dir.into(),
            circuit_breaker,
        }
    }

    /// Get the full path for a relative key.
    fn path(&self, key: &str) -> PathBuf {
        self.root_dir.join(key)
    }

    /// Ensure parent directory exists.
    async fn ensure_dir(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    /// Write bytes atomically (write to temp, then rename).
    async fn write_bytes(&self, key: &str, bytes: &[u8]) -> Result<()> {
        let path = self.path(key);
        self.ensure_dir(&path).await?;

        let tmp = path.with_extension("tmp");
        let mut file = tokio::fs::File::create(&tmp).await?;
        file.write_all(bytes).await?;
        file.flush().await?;
        drop(file);

        tokio::fs::rename(&tmp, &path).await?;
        Ok(())
    }

    /// Write JSON data.
    async fn write_json<T: Serialize + ?Sized>(&self, key: &str, value: &T) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(value)?;
        self.write_bytes(key, &bytes).await
    }

    /// Read bytes, returning None if file doesn't exist.
    async fn read_bytes(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let path = self.path(key);
        match tokio::fs::read(&path).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Io(e)),
        }
    }

    /// Read JSON data.
    async fn read_json<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.read_bytes(key).await? {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Archive key for a given year/month.
    fn archive_key(year: i32, month: u32) -> String {
        format!("stacks/{}/{:02}.json", year, month)
    }

    /// Write hot/cold data and generate index.
    async fn write_hot_cold_data(
        &self,
        outcome: &CrawlOutcome,
        stats: &CrawlStats,
        all_notices: &[NoticeOutput],
        options: &WriteOptions,
    ) -> Result<(usize, usize)> {
        let now = Utc::now();
        let current_year = now.year();
        let current_month = now.month();

        log::info!(
            "Writing {} notices with Hot/Cold partitioning",
            outcome.notices.len()
        );

        // Partition notices by month
        let mut by_month: HashMap<(i32, u32), Vec<NoticeOutput>> = HashMap::new();
        for notice in &outcome.notices {
            let (year, month) = notice.archive_period();
            by_month
                .entry((year, month))
                .or_default()
                .push(NoticeOutput::from(notice));
        }

        // Separate hot (current month) and cold (archived) notices
        let hot_notices: Vec<NoticeOutput> = by_month
            .remove(&(current_year, current_month))
            .unwrap_or_default();

        // Write hot data: current.json
        let current_data = CurrentData::new(hot_notices.clone());
        self.write_json("current.json", &current_data).await?;
        log::info!(
            "Hot data: {} notices written to current.json",
            current_data.count
        );

        // Write cold data: stacks/YYYY/MM.json
        let mut cold_files_updated = 0;
        for ((year, month), notices) in by_month {
            let key = Self::archive_key(year, month);

            // Merge with existing archive if present
            let mut existing: Vec<NoticeOutput> = self.read_json(&key).await?.unwrap_or_default();

            // Deduplicate by ID
            let existing_ids: std::collections::HashSet<_> =
                existing.iter().map(|n| n.id.clone()).collect();

            for notice in notices {
                if !existing_ids.contains(&notice.id) {
                    existing.push(notice);
                }
            }

            // Sort by date descending
            existing.sort_by(|a, b| b.metadata.date.cmp(&a.metadata.date));

            self.write_json(&key, &existing).await?;
            log::info!("Cold data: {} notices written to {}", existing.len(), key);
            cold_files_updated += 1;
        }

        // Generate and write inverted index
        if options.generate_index {
            log::info!(
                "Generating inverted index for {} notices",
                all_notices.len()
            );
            let index = build_index(all_notices);
            self.save_index(&index).await?;
            log::info!(
                "Inverted index: {} tokens indexing {} notices",
                index.token_count,
                index.notice_count
            );
        }

        // Write stats for debugging
        self.write_json("stats.json", stats).await?;

        Ok((current_data.count, cold_files_updated))
    }
}

#[async_trait]
impl NoticeStorage for LocalStorage {
    async fn write_notices(
        &self,
        outcome: &CrawlOutcome,
        campuses: &[Campus],
        stats: &CrawlStats,
    ) -> Result<WriteMetadata> {
        // Use default safe options
        self.write_notices_with_options(outcome, campuses, stats, &WriteOptions::safe())
            .await
    }

    async fn write_notices_with_options(
        &self,
        outcome: &CrawlOutcome,
        _campuses: &[Campus],
        stats: &CrawlStats,
        options: &WriteOptions,
    ) -> Result<WriteMetadata> {
        let now = Utc::now();

        // Convert notices to output format
        let current_notices: Vec<NoticeOutput> =
            outcome.notices.iter().map(NoticeOutput::from).collect();

        // Load previous snapshot for circuit breaker and diff
        let previous_notices = self.load_current().await.unwrap_or_default();

        // Circuit Breaker Check
        if options.circuit_breaker && !options.force_write {
            if let Err(_) = self
                .circuit_breaker
                .validate(&current_notices, &previous_notices)
            {
                log::error!("Circuit breaker triggered - aborting write!");
                return Ok(WriteMetadata {
                    hot_count: 0,
                    cold_files_updated: 0,
                    timestamp: now,
                    diff: None,
                    circuit_breaker_triggered: true,
                });
            }
        }

        // Calculate diff for notifications
        let diff = if options.calculate_diff {
            let diff_result = calculate_diff(&previous_notices, &current_notices);
            if diff_result.has_changes() {
                log::info!(
                    "Diff: {} added, {} updated, {} removed",
                    diff_result.diff.added.len(),
                    diff_result.diff.updated.len(),
                    diff_result.diff.removed.len()
                );
            }
            Some(diff_result)
        } else {
            None
        };

        // Write hot/cold data and generate index
        let (hot_count, cold_files_updated) = self
            .write_hot_cold_data(outcome, stats, &current_notices, options)
            .await?;

        Ok(WriteMetadata {
            hot_count,
            cold_files_updated,
            timestamp: now,
            diff,
            circuit_breaker_triggered: false,
        })
    }

    async fn load_current(&self) -> Result<Vec<NoticeOutput>> {
        match self.read_json::<CurrentData>("current.json").await? {
            Some(data) => Ok(data.notices),
            None => {
                log::warn!("No current.json found");
                Ok(Vec::new())
            }
        }
    }

    async fn load_archive(&self, year: i32, month: u32) -> Result<Vec<NoticeOutput>> {
        let key = Self::archive_key(year, month);
        match self.read_json(&key).await? {
            Some(notices) => Ok(notices),
            None => {
                log::warn!("No archive found for {}/{:02}", year, month);
                Ok(Vec::new())
            }
        }
    }

    async fn load_index(&self) -> Result<Option<InvertedIndex>> {
        self.read_json("index.json").await
    }

    async fn save_index(&self, index: &InvertedIndex) -> Result<()> {
        self.write_json("index.json", index).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NoticeMetadata;
    use crate::pipeline::CircuitBreakerConfig;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_and_read() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path());

        storage.write_bytes("test.txt", b"hello").await.unwrap();
        let data = storage.read_bytes("test.txt").await.unwrap();
        assert_eq!(data, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn test_read_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path());

        let data = storage.read_bytes("nope.txt").await.unwrap();
        assert!(data.is_none());
    }

    #[tokio::test]
    async fn test_current_data_serialization() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path());

        let notices = vec![NoticeOutput {
            id: "yonsei_test_20260201_0001".to_string(),
            title: "Test Notice".to_string(),
            link: "https://example.com/1".to_string(),
            metadata: NoticeMetadata {
                campus: "신촌캠퍼스".to_string(),
                college: "공과대학".to_string(),
                department_name: "테스트학과".to_string(),
                board_name: "공지사항".to_string(),
                date: "2026-02-01".to_string(),
                pinned: false,
            },
        }];

        let current = CurrentData::new(notices);
        storage.write_json("current.json", &current).await.unwrap();

        let loaded: CurrentData = storage.read_json("current.json").await.unwrap().unwrap();

        assert_eq!(loaded.count, 1);
        assert_eq!(loaded.notices[0].id, "yonsei_test_20260201_0001");
    }

    #[tokio::test]
    async fn test_inverted_index_save_load() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorage::new(tmp.path());

        let notices = vec![NoticeOutput {
            id: "001".to_string(),
            title: "장학금 신청 안내".to_string(),
            link: "https://example.com/1".to_string(),
            metadata: NoticeMetadata {
                campus: "신촌캠퍼스".to_string(),
                college: "".to_string(),
                department_name: "학생처".to_string(),
                board_name: "공지".to_string(),
                date: "2026-02-02".to_string(),
                pinned: false,
            },
        }];

        let index = build_index(&notices);
        storage.save_index(&index).await.unwrap();

        let loaded = storage.load_index().await.unwrap().unwrap();
        assert_eq!(loaded.notice_count, 1);
        assert!(loaded.index.contains_key("장학금"));
    }

    #[tokio::test]
    async fn test_circuit_breaker_custom_config() {
        let tmp = TempDir::new().unwrap();
        let config = CircuitBreakerConfig {
            max_drop_percent: 10, // Stricter threshold
            min_baseline: 5,
            allow_cold_start: true,
        };
        let cb = CircuitBreaker::with_config(config);
        let storage = LocalStorage::with_circuit_breaker(tmp.path(), cb);

        // Storage should be created successfully
        assert!(storage.path("test.txt").exists() == false);
    }
}
