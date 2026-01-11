// src/main.rs

//! uRing: Integrated University Notice Crawler CLI
//!
//! This is the main CLI entry point for local development and testing.
//! For AWS Lambda deployment, use the `lambda` binary with the `lambda` feature.

#[cfg(feature = "lambda")]
mod lambda;

mod config;
mod error;
mod models;
mod pipeline;
mod services;
mod storage;
mod utils;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::error::Result;
use crate::models::{Config, LocaleConfig};
use crate::pipeline::{run_archive, run_crawler, run_load, run_mapper, run_pipeline, run_validate};
use crate::utils::log;

#[derive(Parser, Debug)]
#[command(
    name = "uRing",
    version = "1.0.0",
    about = "Integrated University Notice Crawler"
)]

/// CLI Arguments
struct Cli {
    #[arg(short, long, default_value = "data/config.toml")]
    config: String,

    #[arg(long, default_value = "data/locale.toml")]
    locale: String,

    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Command,
}

/// CLI Commands
#[derive(Subcommand, Debug)]
enum Command {
    /// Discover departments and boards
    Map {
        #[arg(long)]
        force: bool,
    },
    /// Fetch notices from discovered boards
    Crawl {
        #[arg(long)]
        site_map: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Validate configuration and seed data
    Validate,
    /// Archive new notices to monthly storage
    Archive,
    /// Load notices from storage
    Load {
        /// Load from "new" storage or specific month (YYYY-MM format)
        #[arg(long, default_value = "new")]
        from: String,
    },
    /// Run the full pipeline
    Pipeline {
        /// Skip the map step (use existing sitemap)
        #[arg(long)]
        skip_map: bool,
    },
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut config = Config::load_or_default(&cli.config);
    let locale = LocaleConfig::load_or_default(&cli.locale);

    // Initialize logging system
    log::init(&locale, &config.logging.level);

    if cli.quiet {
        config.output.console_enabled = false;
        config.logging.show_progress = false;
    }

    match cli.command {
        Command::Map { force: _ } => run_mapper(&config, &locale, &base_path).await?,
        Command::Crawl { site_map, output } => {
            if let Some(path) = site_map {
                config.paths.departments_boards_file = path;
            }
            if let Some(path) = output {
                config.paths.output_dir = path;
            }
            run_crawler(&config, &locale, &base_path).await?;
        }
        Command::Validate => run_validate(&locale, &base_path)?,
        Command::Archive => run_archive(&config, &locale).await?,
        Command::Load { from } => run_load(&config, &locale, &from).await?,
        Command::Pipeline { skip_map } => {
            run_pipeline(&config, &locale, &base_path, skip_map).await?
        }
    }

    Ok(())
}
