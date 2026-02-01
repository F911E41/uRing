//! Storage abstractions for notice persistence.
//!
//! Implements the Hot/Cold storage pattern from README.md:
//! - Hot Data: `current.json` - Latest notices with SWR caching
//! - Cold Data: `stacks/YYYY/MM.json` - Immutable monthly archives
//!
//! ## Directory Structure
//!
//! ```text
//! storage/
//! ├── current.json          # Hot: Latest notices (SWR cached)
//! └── stacks/               # Cold: Monthly archives (immutable)
//!     ├── 2025/
//!     │   ├── 01.json
//!     │   └── 12.json
//!     └── 2026/
//!         └── 01.json
//! ```

pub mod local;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::{Campus, CrawlOutcome, CrawlStats, NoticeOutput};

// Re-export for convenience
pub use local::LocalStorage;

/// Metadata about a storage write operation.
#[derive(Debug, Clone)]
pub struct WriteMetadata {
    /// Number of notices in current.json
    pub hot_count: usize,
    /// Number of archive files updated
    pub cold_files_updated: usize,
    /// Timestamp of the write
    pub timestamp: DateTime<Utc>,
}

/// Header for current.json with cache control hints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentData {
    /// ISO 8601 timestamp of last update
    pub updated_at: DateTime<Utc>,
    /// Total notice count
    pub count: usize,
    /// The notices array
    pub notices: Vec<NoticeOutput>,
}

impl CurrentData {
    pub fn new(notices: Vec<NoticeOutput>) -> Self {
        Self {
            updated_at: Utc::now(),
            count: notices.len(),
            notices,
        }
    }
}

/// Trait for notice storage backends.
#[async_trait]
pub trait NoticeStorage: Send + Sync {
    /// Write notices using Hot/Cold partitioning.
    ///
    /// - Hot: Recent notices go to `current.json`
    /// - Cold: Older notices are archived to `stacks/YYYY/MM.json`
    async fn write_notices(
        &self,
        outcome: &CrawlOutcome,
        campuses: &[Campus],
        stats: &CrawlStats,
    ) -> Result<WriteMetadata>;

    /// Load hot notices from current.json.
    async fn load_current(&self) -> Result<Vec<NoticeOutput>>;

    /// Load archived notices for a specific month.
    async fn load_archive(&self, year: i32, month: u32) -> Result<Vec<NoticeOutput>>;
}
