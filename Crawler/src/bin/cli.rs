//! uRing Crawler CLI
//!
//! Local execution entry point. For AWS Lambda, use `crawler-lambda`.

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use crawler::{
    error::Result,
    models::{Campus, Config},
    pipeline,
    storage::LocalStorage,
    utils::http,
};

/// uRing - University Notice Crawler
#[derive(Parser, Debug)]
#[command(
    name = "uRing",
    version,
    about = "Integrated University Notice Crawler"
)]

struct Cli {
    /// Path to storage directory containing config files
    #[arg(short, long, default_value = "storage")]
    storage_dir: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Discover departments and boards from campuses
    #[cfg(feature = "map")]
    Map {
        /// Force regenerate even if sitemap exists
        #[arg(long)]
        force: bool,
    },

    /// Crawl notices from all discovered boards
    Crawl {
        /// Path to sitemap file (default: {storage_dir}/siteMap.json)
        #[arg(long)]
        sitemap: Option<PathBuf>,
    },

    /// Run full pipeline: Map → Crawl
    #[cfg(feature = "map")]
    Pipeline {
        /// Skip mapping, use existing sitemap
        #[arg(long)]
        skip_map: bool,
    },

    /// Validate configuration files
    Validate,

    /// Show current snapshot info
    Info,
}

/// Initialize logging based on verbosity flag.
fn init_logging(verbose: bool) {
    let level = if verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level))
        .format_timestamp_secs()
        .init();
}

/// Main entry point for the CLI application.
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logging(cli.verbose);

    log::info!("uRing Crawler starting...");

    // Load configurations
    let config_path = cli.storage_dir.join("config.toml");
    let config = Config::load_or_default(&config_path);

    log::info!("Loaded configuration from {}", cli.storage_dir.display());

    let config = Arc::new(config);
    let storage = LocalStorage::new(&cli.storage_dir);
    let sitemap_path = cli.storage_dir.join("siteMap.json");

    match cli.command {
        #[cfg(feature = "map")]
        Command::Map { force } => {
            if sitemap_path.exists() && !force {
                log::warn!(
                    "Sitemap already exists at {}. Use --force to overwrite.",
                    sitemap_path.display()
                );
                return Ok(());
            }

            let client = http::create_client(&config.crawler)?;
            let campuses = pipeline::run_mapper(&config, &client).await?;

            // Save sitemap
            let json = serde_json::to_string_pretty(&campuses)?;
            std::fs::write(&sitemap_path, json)?;

            log::info!("Sitemap saved to {}", sitemap_path.display());
            log::info!(
                "Discovered {} campuses with {} total boards",
                campuses.len(),
                campuses.iter().map(|c| c.board_count()).sum::<usize>()
            );
        }

        Command::Crawl { sitemap } => {
            let sitemap_path = sitemap.unwrap_or(sitemap_path);

            if !sitemap_path.exists() {
                log::error!(
                    "Sitemap not found at {}. Run 'map' first.",
                    sitemap_path.display()
                );
                return Err(crawler::error::AppError::config("Sitemap not found"));
            }

            let campuses = Campus::load_all(&sitemap_path)?;
            log::info!(
                "Loaded {} campuses with {} boards",
                campuses.len(),
                campuses.iter().map(|c| c.board_count()).sum::<usize>()
            );

            let client = http::create_client(&config.crawler)?;
            pipeline::run_crawler(Arc::clone(&config), &storage, &campuses, &client).await?;

            log::info!("Crawl complete!");
        }

        #[cfg(feature = "map")]
        Command::Pipeline { skip_map } => {
            let client = http::create_client(&config.crawler)?;

            // Step 1: Map (unless skipped)
            let campuses = if skip_map {
                if !sitemap_path.exists() {
                    return Err(crawler::error::AppError::config(
                        "Cannot skip map: siteMap.json not found",
                    ));
                }

                log::info!("Skipping map, loading existing sitemap...");
                Campus::load_all(&sitemap_path)?
            } else {
                log::info!("Step 1/2: Mapping departments and boards...");
                let campuses = pipeline::run_mapper(&config, &client).await?;
                let json = serde_json::to_string_pretty(&campuses)?;
                std::fs::write(&sitemap_path, json)?;
                log::info!("Sitemap saved to {}", sitemap_path.display());
                campuses
            };

            // Step 2: Crawl
            log::info!("Step 2/2: Crawling notices...");
            pipeline::run_crawler(Arc::clone(&config), &storage, &campuses, &client).await?;

            log::info!("Pipeline complete!");
        }

        Command::Validate => {
            log::info!("Validating configuration...");

            if let Err(e) = config.validate() {
                log::error!("Config validation failed: {}", e);
                return Err(e);
            }
            log::info!("✓ Config OK (includes campuses, keywords, and CMS patterns)");

            log::info!("All validations passed!");
        }

        Command::Info => {
            log::info!("Storage directory: {}", cli.storage_dir.display());
            log::info!(
                "Sitemap: {}",
                if sitemap_path.exists() {
                    "exists"
                } else {
                    "not found"
                }
            );

            let current_path = cli.storage_dir.join("current.json");
            if current_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&current_path) {
                    if let Ok(pointer) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(version) = pointer.get("version") {
                            log::info!("Current snapshot: {}", version);
                        }
                        if let Some(updated) = pointer.get("updated_at") {
                            log::info!("Last updated: {}", updated);
                        }
                    }
                }
            } else {
                log::info!("No snapshot found yet.");
            }
        }
    }

    log::info!("Done!");

    Ok(())
}
