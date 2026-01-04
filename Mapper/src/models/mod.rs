//! Data models for the Mapper.

pub mod discovery;

mod config;
mod seed;

pub use config::{Config, DiscoveryConfig, HttpConfig};
pub use discovery::{
    Board, BoardDiscoveryResult, Campus, CmsSelectors, College, Department, ManualReviewItem,
};
pub use seed::{CampusInfo, CmsPattern, KeywordMapping, Seed};
