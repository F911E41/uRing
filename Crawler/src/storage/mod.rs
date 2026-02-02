//! Storage abstractions for notice persistence.
//!
//! Implements the Hot/Cold storage pattern:
//!
//! - Hot Data: `current.json` - Latest notices with SWR caching
//! - Cold Data: `stacks/YYYY/MM.json` - Immutable monthly archives
//! - Index: `index.json` - Inverted index for serverless search
//!
//! ## Features
//!
//! - Circuit Breaker: Prevents data corruption on abnormal drops
//! - Diff Calculation: Identifies new/updated/removed notices
//! - Inverted Index: Enables client-side full-text search
//!
//! ## Directory Structure
//!
//! ```text
//! storage/
//! ├── config.toml           # Crawler Configuration
//! ├── index.json            # Inverted Index for Search
//! ├── current.json          # Hot: Latest notices (SWR cached)
//! ├── siteMap.json          # Site Map for Crawling
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
use crate::pipeline::{DiffResult, InvertedIndex};

// Re-export for convenience
pub use local::LocalStorage;

/// Metadata about a storage write operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteMetadata {
    /// Number of notices in current.json
    pub hot_count: usize,
    /// Number of archive files updated
    pub cold_files_updated: usize,
    /// Timestamp of the write
    pub timestamp: DateTime<Utc>,
    /// Diff result (changes from previous snapshot)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffResult>,
    /// Whether circuit breaker was triggered (write aborted)
    #[serde(default)]
    pub circuit_breaker_triggered: bool,
}

/// Options for write operations.
#[derive(Debug, Clone, Default)]
pub struct WriteOptions {
    /// Enable circuit breaker check (default: true)
    pub circuit_breaker: bool,
    /// Generate inverted index (default: true)
    pub generate_index: bool,
    /// Calculate diff from previous snapshot (default: true)
    pub calculate_diff: bool,
    /// Force write even if circuit breaker triggers (USE WITH CAUTION)
    pub force_write: bool,
}

impl WriteOptions {
    /// Create default write options with all safety features enabled.
    pub fn safe() -> Self {
        Self {
            circuit_breaker: true,
            generate_index: true,
            calculate_diff: true,
            force_write: false,
        }
    }

    /// Create write options without safety checks (for testing).
    pub fn unsafe_for_testing() -> Self {
        Self {
            circuit_breaker: false,
            generate_index: false,
            calculate_diff: false,
            force_write: true,
        }
    }
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
    /// Write notices using Hot/Cold partitioning with safety checks.
    ///
    /// This is the primary write method that:
    /// 1. Loads the previous snapshot
    /// 2. Runs circuit breaker validation
    /// 3. Calculates diff for notifications
    /// 4. Writes hot data to `current.json`
    /// 5. Archives cold data to `stacks/YYYY/MM.json`
    /// 6. Generates inverted index to `index.json`
    ///
    /// Returns `WriteMetadata` with operation details and diff result.
    async fn write_notices(
        &self,
        outcome: &CrawlOutcome,
        campuses: &[Campus],
        stats: &CrawlStats,
    ) -> Result<WriteMetadata>;

    /// Write notices with custom options.
    async fn write_notices_with_options(
        &self,
        outcome: &CrawlOutcome,
        campuses: &[Campus],
        stats: &CrawlStats,
        options: &WriteOptions,
    ) -> Result<WriteMetadata>;

    /// Load hot notices from current.json.
    async fn load_current(&self) -> Result<Vec<NoticeOutput>>;

    /// Load archived notices for a specific month.
    async fn load_archive(&self, year: i32, month: u32) -> Result<Vec<NoticeOutput>>;

    /// Load the inverted index.
    async fn load_index(&self) -> Result<Option<InvertedIndex>>;

    /// Save the inverted index.
    async fn save_index(&self, index: &InvertedIndex) -> Result<()>;
}
