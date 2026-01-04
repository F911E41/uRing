//! Service modules for crawling and discovery.

mod boards;
mod departments;
mod selectors;

pub use boards::BoardDiscoveryService;
pub use departments::DepartmentCrawler;
pub use selectors::SelectorDetector;
