//! Crawler modules for Yonsei University data.

pub mod boards;
pub mod departments;

pub use boards::discover_boards;
pub use departments::crawl_all_campuses;
