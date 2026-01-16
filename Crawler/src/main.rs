// src/main.rs

//! uRing: Integrated University Notice Crawler CLI
//!
//! This is the main CLI entry point for local development and testing.
//! For AWS Lambda deployment, use the `lambda` binary with the `lambda` feature.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use crawler::{
    error::{AppError, Result},
    models::{Campus, Config, LocaleConfig, Seed},
    pipeline::{crawl::run_crawler, map::run_mapper, run_pipeline},
    storage::{NoticeStorage, s3::S3Storage},
    utils::{fs, http, log},
};

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

    #[arg(long, default_value = "data/seed.toml")]
    seed: String,

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
    /// Load notices from storage
    Load {
        /// Load from "new" snapshot or specific month (YYYY-MM format)
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

    let mut config = Config::load_or_default(&cli.config);
    let locale = LocaleConfig::load_or_default(&cli.locale);
    let seed = Seed::load(&cli.seed)?;

    // Initialize logging system
    log::init(&locale, &config.logging.level);

    if cli.quiet {
        config.output.console_enabled = false;
        config.logging.show_progress = false;
    }

    match cli.command {
        Command::Map { force } => {
            let base = std::env::current_dir()?;
            let site_map_path = config.departments_boards_path(&base);

            if site_map_path.exists() && !force {
                log::warn(&format!(
                    "siteMap already exists at {} (use --force to overwrite)",
                    site_map_path.display()
                ));
                return Ok(());
            }

            let client = http::create_async_client(&config.crawler)?;
            let campuses = run_mapper(&config, &locale, &seed, &client).await?;
            fs::save_json(&site_map_path, &campuses)?;

            log::success(
                &locale
                    .messages
                    .mapper_complete
                    .replace("{path}", &site_map_path.display().to_string()),
            );
        }
        Command::Crawl { site_map, output } => {
            let base = std::env::current_dir()?;
            let site_map_path = site_map
                .map(PathBuf::from)
                .unwrap_or_else(|| config.departments_boards_path(&base));

            log::info(
                &locale
                    .messages
                    .crawler_loading_sitemap
                    .replace("{path}", &site_map_path.display().to_string()),
            );

            let campuses = Campus::load_all(&site_map_path)?;
            let storage = S3Storage::from_env().await?;
            run_crawler(&config, &locale, &storage, &campuses).await?;

            if let Some(path) = output {
                log::warn(&format!("Output path {} is not used with S3 storage", path));
            }
        }
        Command::Validate => {
            log::header(&locale.messages.validate_starting);
            log::success(&locale.messages.validate_config_success);

            if let Err(err) = seed.validate() {
                log::error(
                    &locale
                        .messages
                        .validate_failed
                        .replace("{error}", &err.to_string()),
                );
                return Err(err);
            }

            log::success(&locale.messages.validate_seed_success);
            log::info(
                &locale
                    .messages
                    .validate_user_agent
                    .replace("{value}", &config.crawler.user_agent),
            );
            log::info(
                &locale
                    .messages
                    .validate_timeout
                    .replace("{value}", &config.crawler.timeout_secs.to_string()),
            );
            log::info(
                &locale
                    .messages
                    .validate_max_concurrent
                    .replace("{value}", &config.crawler.max_concurrent.to_string()),
            );
            log::info(
                &locale
                    .messages
                    .validate_campuses
                    .replace("{count}", &seed.campuses.len().to_string()),
            );
            log::info(
                &locale
                    .messages
                    .validate_keywords
                    .replace("{count}", &seed.keywords.len().to_string()),
            );
            log::info(
                &locale
                    .messages
                    .validate_patterns
                    .replace("{count}", &seed.cms_patterns.len().to_string()),
            );
        }
        Command::Load { from } => {
            if from != "new" {
                return Err(AppError::config(format!("Unsupported load target: {from}")));
            }

            let storage = S3Storage::from_env().await?;
            log::info(&locale.messages.load_new);
            let notices = storage.load_snapshot().await?;

            log::success(
                &locale
                    .messages
                    .load_complete
                    .replace("{count}", &notices.len().to_string()),
            );

            if config.output.console_enabled {
                for item in notices {
                    log::info(
                        &locale
                            .messages
                            .load_notice_item
                            .replace("{title}", &item.title)
                            .replace("{date}", &item.date),
                    );
                }
            }
        }
        Command::Pipeline { skip_map } => {
            let storage = S3Storage::from_env().await?;

            if skip_map {
                let base = std::env::current_dir()?;
                let site_map_path = config.departments_boards_path(&base);
                log::info(
                    &locale
                        .messages
                        .crawler_loading_sitemap
                        .replace("{path}", &site_map_path.display().to_string()),
                );

                let campuses = Campus::load_all(&site_map_path)?;
                storage
                    .write_config_bundle(&config, &seed, &campuses)
                    .await?;
                run_crawler(&config, &locale, &storage, &campuses).await?;
            } else {
                run_pipeline(&config, &locale, &seed, &storage).await?;
            }
        }
    }

    Ok(())
}
