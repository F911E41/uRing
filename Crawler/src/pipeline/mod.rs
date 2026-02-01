//! Pipeline entry points for crawler operations.
//!
//! - `run_mapper`: Discover departments and boards from campus URLs
//! - `run_crawler`: Fetch notices from discovered boards

pub mod crawl;
#[cfg(feature = "map")]
pub mod map;

pub use crawl::run_crawler;
#[cfg(feature = "map")]
pub use map::run_mapper;
