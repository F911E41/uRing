// src/pipeline/mod.rs

//! Pipeline entry points for CLI commands.

pub mod archive;
pub mod crawl;
pub mod load;
pub mod map;
pub mod pipeline;
pub mod validate;

pub use archive::run_archive;
pub use crawl::run_crawler;
pub use load::run_load;
pub use map::run_mapper;
pub use pipeline::run_pipeline;
pub use validate::run_validate;
