//! Pipeline entry points for crawler operations.
//!
//! ## Available Operations
//!
//! - `run_mapper`: Discover departments and boards from campus URLs
//! - `run_crawler`: Fetch notices from discovered boards
//! - `circuit_breaker`: Prevent data corruption on abnormal drops
//! - `diff`: Calculate changes between snapshots for notifications
//! - `index`: Build inverted index for serverless search

pub mod circuit_breaker;
pub mod crawl;
pub mod diff;
pub mod index;

#[cfg(feature = "map")]
pub mod map;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerResult};
pub use crawl::run_crawler;
pub use diff::{DiffCalculator, DiffResult, calculate_diff};
pub use index::{IndexBuilder, IndexConfig, InvertedIndex, build_index};

#[cfg(feature = "map")]
pub use map::run_mapper;
