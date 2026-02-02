//! uRing Crawler Library
//!
//! A modular web crawler for university notice boards.
//!
//! ## Architecture Overview
//!
//! - `models`: Data structures (Config, Campus, Notice, etc.)
//! - `services`: Business logic (crawlers, parsers, detectors)
//! - `pipeline`: High-level operations (map, crawl, circuit_breaker, diff, index)
//! - `storage`: Persistence backends (local filesystem, S3)
//! - `utils`: Shared utilities (HTTP client)
//! - `error`: Unified error handling
//!
//! ## Key Features
//!
//! - **Circuit Breaker**: Prevents data corruption when source sites return empty/partial data
//! - **Inverted Index**: Enables client-side full-text search without a backend search engine
//! - **Diff Calculation**: Identifies new/updated/removed notices for event-driven notifications
//! - **Hot/Cold Storage**: Efficient data partitioning for CDN caching

pub mod error;
pub mod models;
pub mod pipeline;
pub mod services;
pub mod storage;
pub mod utils;

// Re-export commonly used items
pub use error::{AppError, Result};

// Re-export pipeline components
pub use pipeline::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerResult, DiffCalculator, DiffResult,
    IndexBuilder, IndexConfig, InvertedIndex, build_index, calculate_diff,
};

// Re-export storage components
pub use storage::{LocalStorage, NoticeStorage, WriteMetadata, WriteOptions};
