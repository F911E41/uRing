// src/pipeline/mod.rs

//! Pipeline entry points for CLI commands.

pub mod crawl;
pub mod map;
pub mod pipeline;

pub use pipeline::run_pipeline;
