//! uRing Crawler Library
//!
//! A modular web crawler for university notice boards.
//!
//! # Architecture
//!
//! - `models`: Data structures (Config, Campus, Notice, Seed)
//! - `services`: Business logic (crawlers, parsers, detectors)
//! - `pipeline`: High-level operations (map, crawl)
//! - `storage`: Persistence backends (local filesystem, S3)
//! - `utils`: Shared utilities (HTTP client)
//! - `error`: Unified error handling

pub mod error;
pub mod models;
pub mod pipeline;
pub mod services;
pub mod storage;
pub mod utils;

// Re-export commonly used items
pub use error::{AppError, Result};
