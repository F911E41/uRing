// src/storage/s3.rs

//! AWS S3 storage implementation.
//!
//! Implements the append-only event log + snapshot pointer approach:
//! - Events are stored at `{bucket}/{campus}/events/YYYY-MM/{notice_id}.json`
//! - Snapshots are stored at `{bucket}/{campus}/snapshots/{timestamp}.json`
//! - Pointer is stored at `{bucket}/{campus}/new.pointer.json`

use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;

use chrono::{Datelike, NaiveDate, Utc};
use serde::de::DeserializeOwned;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::Notice;
use crate::storage::{
    EventStorageSummary, NoticeStorage, SnapshotMetadata, SnapshotPointer, paths,
};

/// S3-based notice storage implementing the append-only approach.
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

    /// Create a new storage instance scoped to a campus-specific prefix.
    pub fn with_campus(&self, campus: &str) -> Self {
        let campus_prefix = paths::campus_prefix(&self.prefix, campus);
        Self {
            client: self.client.clone(),
            bucket: self.bucket.clone(),
            prefix: campus_prefix,
        }
    }

    /// Create S3 storage from environment configuration.
    pub async fn from_env() -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);

        let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uring-notices".to_string());
        let prefix = std::env::var("S3_PREFIX").unwrap_or_else(|_| "uRing".to_string());

        Ok(Self::new(client, bucket, prefix))
    }

    /// Read JSON from S3, returning None if the key does not exist.
    pub async fn read_json_optional<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
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
                    AppError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ))
                })?;
                let value: T = serde_json::from_slice(&bytes.into_bytes())?;
                Ok(Some(value))
            }
            Err(err) => {
                let service_err = err.into_service_error();
                if service_err.is_no_such_key() {
                    info!("No existing data at s3://{}/{}", self.bucket, key);
                    Ok(None)
                } else {
                    Err(AppError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        service_err.to_string(),
                    )))
                }
            }
        }
    }

    /// Read notices JSON from S3.
    async fn read_json(&self, key: &str) -> Result<Vec<Notice>> {
        Ok(self
            .read_json_optional::<Vec<Notice>>(key)
            .await?
            .unwrap_or_default())
    }

    /// Read pointer JSON from S3.
    async fn read_pointer(&self, key: &str) -> Result<Option<SnapshotPointer>> {
        self.read_json_optional::<SnapshotPointer>(key).await
    }

    /// Write JSON to S3.
    async fn write_json_bytes(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        let body = ByteStream::from(bytes);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        info!("Wrote object to s3://{}/{}", self.bucket, key);
        Ok(())
    }

    /// Write a notice JSON to S3.
    async fn write_notice(&self, key: &str, notice: &Notice) -> Result<()> {
        let json = serde_json::to_string_pretty(notice)?;
        self.write_json_bytes(key, json.into_bytes()).await
    }

    /// Write a snapshot JSON to S3.
    async fn write_snapshot_file(&self, key: &str, notices: &[Notice]) -> Result<()> {
        let json = serde_json::to_string_pretty(notices)?;
        self.write_json_bytes(key, json.into_bytes()).await
    }

    /// Check if an object exists in S3.
    async fn exists(&self, key: &str) -> bool {
        self.client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .is_ok()
    }

    /// List all objects with a prefix.
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut continuation: Option<String> = None;

        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix);
            if let Some(token) = continuation.clone() {
                request = request.continuation_token(token);
            }

            let response = request.send().await.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

            if let Some(contents) = response.contents {
                for item in contents {
                    if let Some(key) = item.key {
                        keys.push(key);
                    }
                }
            }

            continuation = response.next_continuation_token;
            if continuation.is_none() {
                break;
            }
        }

        Ok(keys)
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
}

impl NoticeStorage for S3Storage {
    /// Store notices in the append-only event log.
    async fn store_events(&self, notices: &[Notice]) -> Result<EventStorageSummary> {
        let mut stored = Vec::new();
        let mut skipped = 0;

        for notice in notices {
            let (year, month) = Self::notice_month(notice);
            let notice_id = notice.canonical_id();
            let key = paths::event_key(&self.prefix, year, month, &notice_id);

            if self.exists(&key).await {
                skipped += 1;
                continue;
            }

            self.write_notice(&key, notice).await?;
            stored.push(notice.clone());
        }

        Ok(EventStorageSummary {
            stored_notices: stored,
            skipped_count: skipped,
        })
    }

    /// Write a snapshot and update pointer.
    async fn write_snapshot(&self, notices: &[Notice]) -> Result<SnapshotMetadata> {
        let timestamp = Utc::now();
        let snapshot_key = paths::snapshot_key(&self.prefix, timestamp);
        let pointer_key = paths::pointer_key(&self.prefix);

        self.write_snapshot_file(&snapshot_key, notices).await?;

        let pointer = SnapshotPointer::new(snapshot_key.clone());
        let pointer_json = serde_json::to_string_pretty(&pointer)?;
        self.write_json_bytes(&pointer_key, pointer_json.into_bytes())
            .await?;

        Ok(SnapshotMetadata {
            notice_count: notices.len(),
            timestamp,
            snapshot_location: format!("s3://{}/{}", self.bucket, snapshot_key),
            pointer_location: format!("s3://{}/{}", self.bucket, pointer_key),
        })
    }

    /// Load notices from the latest snapshot pointer.
    async fn load_snapshot(&self) -> Result<Vec<Notice>> {
        let pointer_key = paths::pointer_key(&self.prefix);
        let pointer = match self.read_pointer(&pointer_key).await? {
            Some(pointer) => pointer,
            None => return Ok(Vec::new()),
        };

        self.read_json(&pointer.snapshot_key).await
    }

    /// Load notices from events for a specific month.
    async fn load_events(&self, year: i32, month: u32) -> Result<Vec<Notice>> {
        let prefix = paths::events_prefix(&self.prefix, year, month);
        let keys = self.list_keys(&prefix).await?;
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut notices = Vec::new();
        for key in keys {
            if !key.ends_with(".json") {
                continue;
            }
            let content = self.read_json_optional::<Notice>(&key).await?;
            if let Some(notice) = content {
                notices.push(notice);
            }
        }

        if notices.is_empty() {
            warn!("No notices found under prefix {}", prefix);
        }

        Ok(notices)
    }
}
