//! AWS S3 storage implementation.
//!
//! Implements the Delta-First approach for notice storage:
//! - New notices are stored in `{bucket}/New/notices.json`
//! - On rotation, content is moved to monthly archive `{bucket}/YYYY-MM/notices.json`

use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use chrono::{DateTime, Utc};
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::Notice;
use crate::storage::{paths, NoticeStorage, StorageMetadata};

/// S3-based notice storage implementing the Delta-First approach.
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

        let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uring-notices".to_string());
        let prefix = std::env::var("S3_PREFIX").unwrap_or_else(|_| "uRing".to_string());

        Ok(Self::new(client, bucket, prefix))
    }

    /// Read JSON from S3.
    async fn read_json(&self, key: &str) -> Result<Vec<Notice>> {
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
                let notices: Vec<Notice> = serde_json::from_slice(&bytes.into_bytes())?;
                Ok(notices)
            }
            Err(err) => {
                // Check if it's a "not found" error
                let service_err = err.into_service_error();
                if service_err.is_no_such_key() {
                    info!("No existing data at s3://{}/{}", self.bucket, key);
                    Ok(Vec::new())
                } else {
                    Err(AppError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        service_err.to_string(),
                    )))
                }
            }
        }
    }

    /// Write JSON to S3.
    async fn write_json(&self, key: &str, notices: &[Notice]) -> Result<()> {
        let json = serde_json::to_string_pretty(notices)?;
        let bytes = ByteStream::from(json.into_bytes());

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(bytes)
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        info!("Wrote {} notices to s3://{}/{}", notices.len(), self.bucket, key);
        Ok(())
    }

    /// Delete an object from S3 (used after rotation).
    async fn delete_object(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        info!("Deleted s3://{}/{}", self.bucket, key);
        Ok(())
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
}

impl NoticeStorage for S3Storage {
    /// Store notices in the "New" directory.
    ///
    /// This is the hot data used for triggering notifications.
    async fn store_new(&self, notices: &[Notice]) -> Result<StorageMetadata> {
        let key = paths::new_notices_key(&self.prefix);
        let timestamp = Utc::now();

        self.write_json(&key, notices).await?;

        Ok(StorageMetadata {
            notice_count: notices.len(),
            timestamp,
            location: format!("s3://{}/{}", self.bucket, key),
        })
    }

    /// Rotate notices from "New" to monthly archive.
    ///
    /// This performs an atomic rotation:
    /// 1. Read existing notices from New/
    /// 2. Merge with existing monthly archive (if any)
    /// 3. Write to monthly archive
    /// 4. Delete New/ (will be overwritten by next crawl)
    async fn rotate_to_archive(&self) -> Result<StorageMetadata> {
        let now = Utc::now();
        let new_key = paths::new_notices_key(&self.prefix);
        let archive_key = paths::monthly_archive_key(&self.prefix, now);

        // Read current "New" notices
        let new_notices = self.read_json(&new_key).await?;

        if new_notices.is_empty() {
            warn!("No notices in New/ to rotate");
            return Ok(StorageMetadata {
                notice_count: 0,
                timestamp: now,
                location: format!("s3://{}/{}", self.bucket, archive_key),
            });
        }

        // Read existing archive (if any)
        let mut archive_notices = self.read_json(&archive_key).await?;
        let initial_count = archive_notices.len();

        // Merge: append new notices, deduplicate by link
        let existing_links: std::collections::HashSet<_> =
            archive_notices.iter().map(|n| &n.link).collect();

        let unique_new: Vec<_> = new_notices
            .into_iter()
            .filter(|n| !existing_links.contains(&n.link))
            .collect();

        let added_count = unique_new.len();
        archive_notices.extend(unique_new);

        // Sort by date descending
        archive_notices.sort_by(|a, b| b.date.cmp(&a.date));

        // Write merged archive
        self.write_json(&archive_key, &archive_notices).await?;

        info!(
            "Rotated {} new notices to archive (total: {}, added: {})",
            added_count,
            archive_notices.len(),
            added_count
        );

        // Clear New/ by deleting it (optional, or leave for overwrite)
        if self.exists(&new_key).await {
            self.delete_object(&new_key).await?;
        }

        Ok(StorageMetadata {
            notice_count: added_count,
            timestamp: now,
            location: format!("s3://{}/{}", self.bucket, archive_key),
        })
    }

    /// Load notices from the "New" directory.
    async fn load_new(&self) -> Result<Vec<Notice>> {
        let key = paths::new_notices_key(&self.prefix);
        self.read_json(&key).await
    }

    /// Load notices from a specific month's archive.
    async fn load_archive(&self, year: i32, month: u32) -> Result<Vec<Notice>> {
        let key = format!(
            "{}/{:04}-{:02}/notices.json",
            self.prefix.trim_end_matches('/'),
            year,
            month
        );
        self.read_json(&key).await
    }
}

/// Delta detection: find notices that are new compared to previous data.
pub fn detect_delta(current: &[Notice], previous: &[Notice]) -> Vec<Notice> {
    let previous_links: std::collections::HashSet<_> = previous.iter().map(|n| &n.link).collect();

    current
        .iter()
        .filter(|n| !previous_links.contains(&n.link))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_delta() {
        let previous = vec![Notice {
            campus: "신촌".to_string(),
            college: "공대".to_string(),
            department_id: "cse".to_string(),
            department_name: "컴퓨터".to_string(),
            board_id: "notice".to_string(),
            board_name: "공지사항".to_string(),
            title: "Old Notice".to_string(),
            date: "2025-01-01".to_string(),
            link: "https://example.com/old".to_string(),
        }];

        let current = vec![
            Notice {
                campus: "신촌".to_string(),
                college: "공대".to_string(),
                department_id: "cse".to_string(),
                department_name: "컴퓨터".to_string(),
                board_id: "notice".to_string(),
                board_name: "공지사항".to_string(),
                title: "Old Notice".to_string(),
                date: "2025-01-01".to_string(),
                link: "https://example.com/old".to_string(),
            },
            Notice {
                campus: "신촌".to_string(),
                college: "공대".to_string(),
                department_id: "cse".to_string(),
                department_name: "컴퓨터".to_string(),
                board_id: "notice".to_string(),
                board_name: "공지사항".to_string(),
                title: "New Notice".to_string(),
                date: "2025-01-11".to_string(),
                link: "https://example.com/new".to_string(),
            },
        ];

        let delta = detect_delta(&current, &previous);
        assert_eq!(delta.len(), 1);
        assert_eq!(delta[0].title, "New Notice");
    }
}
