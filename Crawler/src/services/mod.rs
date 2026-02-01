//! Service layer for the crawler application.
//!
//! This module contains the business logic for:
//! - Board discovery (`BoardDiscoveryService`)
//! - Department crawling (`DepartmentCrawler`)
//! - Notice fetching (`NoticeCrawler`)
//! - CMS selector detection (`SelectorDetector`)

#[cfg(feature = "map")]
mod boards;
#[cfg(feature = "map")]
mod departments;
mod notices;
mod selectors;

#[cfg(feature = "map")]
pub use boards::BoardDiscoveryService;
#[cfg(feature = "map")]
pub use departments::DepartmentCrawler;
pub use notices::NoticeCrawler;
pub use selectors::SelectorDetector;
