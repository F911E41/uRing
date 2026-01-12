// src/storage/mod.rs

//! Storage abstractions for notice persistence.
//!
//! This module provides a unified interface for storing notices,
//! with implementations for local filesystem and AWS S3.

#[cfg(feature = "lambda")]
pub mod s3;

use std::future::Future;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::Notice;

/// Summary for event storage writes.
#[derive(Debug, Clone)]
pub struct EventStorageSummary {
    /// Notices that were newly stored.
    pub stored_notices: Vec<Notice>,
    /// Notices that already existed.
    pub skipped_count: usize,
}

impl EventStorageSummary {
    /// Total number of notices considered.
    pub fn total_count(&self) -> usize {
        self.stored_notices.len() + self.skipped_count
    }
}

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
    pub snapshot_key: String,
    pub updated_at: DateTime<Utc>,
}

impl SnapshotPointer {
    pub fn new(snapshot_key: String) -> Self {
        Self {
            snapshot_key,
            updated_at: Utc::now(),
        }
    }
}

/// Trait for notice storage backends.
pub trait NoticeStorage {
    /// Store notices as append-only events.
    fn store_events(
        &self,
        notices: &[Notice],
    ) -> impl Future<Output = Result<EventStorageSummary>> + Send;

    /// Write a snapshot of new notices and update the pointer.
    fn write_snapshot(
        &self,
        notices: &[Notice],
    ) -> impl Future<Output = Result<SnapshotMetadata>> + Send;

    /// Load notices from the latest snapshot pointer.
    fn load_snapshot(&self) -> impl Future<Output = Result<Vec<Notice>>> + Send;

    /// Load notices from event storage for a specific month.
    fn load_events(
        &self,
        year: i32,
        month: u32,
    ) -> impl Future<Output = Result<Vec<Notice>>> + Send;
}

/// Local filesystem storage implementation.
#[derive(Clone, Debug)]
pub struct LocalStorage {
    base_path: PathBuf,
    campus: Option<String>,
}

impl LocalStorage {
    /// Create a new local storage with the given base path.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            campus: None,
        }
    }

    /// Scope storage to a campus.
    pub fn with_campus(&self, campus: impl Into<String>) -> Self {
        Self {
            base_path: self.base_path.clone(),
            campus: Some(campus.into()),
        }
    }

    fn campus_root(&self) -> PathBuf {
        if let Some(campus) = &self.campus {
            self.base_path.join(campus)
        } else {
            self.base_path.clone()
        }
    }

    fn events_dir(&self, year: i32, month: u32) -> PathBuf {
        self.campus_root()
            .join("events")
            .join(format!("{:04}-{:02}", year, month))
    }

    fn event_path(&self, notice_id: &str, year: i32, month: u32) -> PathBuf {
        self.events_dir(year, month)
            .join(format!("{notice_id}.json"))
    }

    fn snapshots_dir(&self) -> PathBuf {
        self.campus_root().join("snapshots")
    }

    fn snapshot_path(&self, timestamp: DateTime<Utc>) -> PathBuf {
        let file_name = timestamp.format("%Y-%m-%dT%H-%M-%SZ").to_string();
        self.snapshots_dir().join(format!("{file_name}.json"))
    }

    fn pointer_path(&self) -> PathBuf {
        self.campus_root().join("new.pointer.json")
    }

    fn notice_month(notice: &Notice) -> (i32, u32) {
        for fmt in ["%Y-%m-%d", "%Y.%m.%d", "%Y/%m/%d"] {
            if let Ok(date) = NaiveDate::parse_from_str(&notice.date, fmt) {
                return (date.year(), date.month());
            }
        }
        let now = Utc::now();
        (now.year(), now.month())
    }

    fn read_notice_file(path: &Path) -> Result<Notice> {
        let content = std::fs::read_to_string(path)?;
        let notice: Notice = serde_json::from_str(&content)?;
        Ok(notice)
    }

    fn load_events_from_dir(&self, dir: &Path) -> Result<Vec<Notice>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut notices = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                notices.push(Self::read_notice_file(&path)?);
            }
        }
        Ok(notices)
    }

    fn load_events_for_month(&self, year: i32, month: u32) -> Result<Vec<Notice>> {
        let mut notices = Vec::new();
        let root = self.campus_root();

        if self.campus.is_some() {
            return self.load_events_from_dir(&self.events_dir(year, month));
        }

        if !root.exists() {
            return Ok(Vec::new());
        }

        if root.join("events").exists() {
            notices.extend(self.load_events_from_dir(&self.events_dir(year, month))?);
        }

        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let campus_dir = path.join("events").join(format!("{:04}-{:02}", year, month));
            notices.extend(self.load_events_from_dir(&campus_dir)?);
        }

        Ok(notices)
    }
}

impl NoticeStorage for LocalStorage {
    async fn store_events(&self, notices: &[Notice]) -> Result<EventStorageSummary> {
        let mut stored = Vec::new();
        let mut skipped = 0;

        for notice in notices {
            let (year, month) = Self::notice_month(notice);
            let notice_id = notice.canonical_id();
            let path = self.event_path(&notice_id, year, month);

            if path.exists() {
                skipped += 1;
                continue;
            }

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let json = serde_json::to_string_pretty(notice)?;
            std::fs::write(&path, json)?;
            stored.push(notice.clone());
        }

        Ok(EventStorageSummary {
            stored_notices: stored,
            skipped_count: skipped,
        })
    }

    async fn write_snapshot(&self, notices: &[Notice]) -> Result<SnapshotMetadata> {
        let timestamp = Utc::now();
        let snapshot_path = self.snapshot_path(timestamp);
        if let Some(parent) = snapshot_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(notices)?;
        std::fs::write(&snapshot_path, json)?;

        let pointer_path = self.pointer_path();
        if let Some(parent) = pointer_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let pointer = SnapshotPointer::new(snapshot_path.to_string_lossy().to_string());
        let pointer_json = serde_json::to_string_pretty(&pointer)?;
        std::fs::write(&pointer_path, pointer_json)?;

        Ok(SnapshotMetadata {
            notice_count: notices.len(),
            timestamp,
            snapshot_location: snapshot_path.display().to_string(),
            pointer_location: pointer_path.display().to_string(),
        })
    }

    async fn load_snapshot(&self) -> Result<Vec<Notice>> {
        let pointer_path = self.pointer_path();
        if !pointer_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(pointer_path)?;
        let pointer: SnapshotPointer = serde_json::from_str(&content)?;
        let snapshot_path = PathBuf::from(pointer.snapshot_key);
        if !snapshot_path.exists() {
            return Ok(Vec::new());
        }

        let snapshot_content = std::fs::read_to_string(snapshot_path)?;
        let notices: Vec<Notice> = serde_json::from_str(&snapshot_content)?;
        Ok(notices)
    }

    async fn load_events(&self, year: i32, month: u32) -> Result<Vec<Notice>> {
        self.load_events_for_month(year, month)
    }
}

/// S3 path utilities.
pub mod paths {
    use chrono::{DateTime, Utc};

    /// Get the campus-specific prefix (e.g., `uRing/신촌캠퍼스`).
    pub fn campus_prefix(bucket_prefix: &str, campus: &str) -> String {
        let base = bucket_prefix.trim_end_matches('/');
        let campus = campus.trim_matches('/');
        if campus.is_empty() {
            base.to_string()
        } else {
            format!("{}/{}", base, campus)
        }
    }

    /// Get the S3 key for an event notice.
    pub fn event_key(bucket_prefix: &str, year: i32, month: u32, notice_id: &str) -> String {
        format!(
            "{}/events/{:04}-{:02}/{}.json",
            bucket_prefix.trim_end_matches('/'),
            year,
            month,
            notice_id
        )
    }

    /// Get the S3 key for the snapshot file.
    pub fn snapshot_key(bucket_prefix: &str, timestamp: DateTime<Utc>) -> String {
        let file_name = timestamp.format("%Y-%m-%dT%H-%M-%SZ");
        format!(
            "{}/snapshots/{}.json",
            bucket_prefix.trim_end_matches('/'),
            file_name
        )
    }

    /// Get the S3 key for the pointer file.
    pub fn pointer_key(bucket_prefix: &str) -> String {
        format!("{}/new.pointer.json", bucket_prefix.trim_end_matches('/'))
    }

    /// Get the prefix for events for a specific month.
    pub fn events_prefix(bucket_prefix: &str, year: i32, month: u32) -> String {
        format!(
            "{}/events/{:04}-{:02}/",
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
    fn test_campus_prefix() {
        assert_eq!(campus_prefix("uRing", "CampusA"), "uRing/CampusA");
        assert_eq!(campus_prefix("uRing/", "/CampusA"), "uRing/CampusA");
        assert_eq!(campus_prefix("uRing/", ""), "uRing");
    }

    #[test]
    fn test_event_key() {
        assert_eq!(
            event_key("uRing", 2025, 1, "abc123"),
            "uRing/events/2025-01/abc123.json"
        );
    }

    #[test]
    fn test_snapshot_key() {
        let ts = chrono::Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
        assert_eq!(
            snapshot_key("uRing", ts),
            "uRing/snapshots/2025-01-15T12-00-00Z.json"
        );
    }

    #[test]
    fn test_pointer_key() {
        assert_eq!(pointer_key("uRing"), "uRing/new.pointer.json");
        assert_eq!(pointer_key("uRing/"), "uRing/new.pointer.json");
    }

    #[test]
    fn test_events_prefix() {
        assert_eq!(events_prefix("uRing", 2025, 1), "uRing/events/2025-01/");
    }
}
