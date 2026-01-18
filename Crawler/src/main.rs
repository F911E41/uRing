// src/main.rs

//! uRing: Integrated University Notice Crawler CLI
//!
//! This is the main CLI entry point for local development and testing.
//! For AWS Lambda deployment, use the `lambda` binary with the `lambda` feature.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use crawler::{
    error::{AppError, Result},
    models::{Campus, Config, LocaleConfig, Seed},
    pipeline::{crawl::run_crawler, map::run_mapper},
    storage::{NoticeStorage, local::LocalStorage},
    utils::{fs, http, log},
};

#[cfg(feature = "s3")]
use crawler::storage::s3::S3Storage;

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

    /// Select storage backend (local fs or aws s3)
    #[arg(long, global = true, default_value = "s3")]
    storage: StorageMode,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, ValueEnum)]
enum StorageMode {
    Local,
    S3,
}

/// CLI Commands
#[derive(Subcommand, Debug)]
enum Command {
    /// Discover departments and boards
    Map {
        /// Force regenerate the site map even if it exists
        #[arg(long)]
        force: bool,

        /// Regenerate if the existing site map is older than N days
        #[arg(long)]
        refresh_days: Option<u64>,
    },
    /// Fetch notices from discovered boards
    Crawl {
        /// Optional path to a specific site map file
        #[arg(long)]
        site_map: Option<String>,
    },
    /// Validate configuration and seed data
    Validate,
    /// Load notices from storage
    Load {
        /// Load from "new" snapshot or specific month (YYYY-MM format)
        #[arg(long, default_value = "new")]
        from: LoadFrom,
    },
    /// Run the full pipeline (Map -> Bundle -> Crawl)
    Pipeline {
        /// Skip the map step (use existing sitemap)
        #[arg(long)]
        skip_map: bool,
    },
}

#[derive(Clone, Debug)]
enum LoadFrom {
    New,
    Month(String),
}

impl std::str::FromStr for LoadFrom {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if value == "new" {
            return Ok(LoadFrom::New);
        }
        if is_valid_month(value) {
            return Ok(LoadFrom::Month(value.to_string()));
        }
        Err(format!(
            "Invalid load target: {value}. Use 'new' or YYYY-MM."
        ))
    }
}

fn is_valid_month(value: &str) -> bool {
    let mut parts = value.split('-');
    let year = parts.next().unwrap_or("");
    let month = parts.next().unwrap_or("");
    if parts.next().is_some() {
        return false;
    }
    if year.len() != 4 || month.len() != 2 {
        return false;
    }
    if !year.chars().all(|c| c.is_ascii_digit()) || !month.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    matches!(month.parse::<u8>(), Ok(1..=12))
}

/// Helper to check if a file is stale (older than `days`)
fn is_stale(path: &Path, days: u64) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };

    let now = std::time::SystemTime::now();
    let age = match now.duration_since(modified) {
        Ok(d) => d,
        Err(_) => return true, // Modified time is in the future
    };

    age.as_secs() > 60 * 60 * 24 * days
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut config = Config::load_or_default(&cli.config);
    let locale = LocaleConfig::load_or_default(&cli.locale);
    let seed = Seed::load(&cli.seed)?;

    // Handle quiet flag BEFORE log::init
    // Adjust config based on quiet flag so init receives correct level
    if cli.quiet {
        config.output.console_enabled = false;
        config.logging.show_progress = false;
        // Assuming log::init uses the config's level, we temporarily override or pass a silencer.
        // Here, we assume config.logging.level controls verbosity.
        // If LogLevel is an enum, set it to Error or Off.
        // config.logging.level = LogLevel::Error;
    }

    // Initialize logging system with the adjusted configuration
    log::init(&locale, &config.logging.level);

    let config = Arc::new(config);

    match cli.command {
        Command::Map {
            force,
            refresh_days,
        } => {
            let base = std::env::current_dir()?;
            let site_map_path = config.departments_boards_path(&base);

            // Handle staleness or existence
            let should_run = if force {
                true
            } else if !site_map_path.exists() {
                true
            } else if let Some(days) = refresh_days {
                if is_stale(&site_map_path, days) {
                    log::warn(&format!(
                        "Site map is older than {} days. Refreshing...",
                        days
                    ));
                    true
                } else {
                    false
                }
            } else {
                false
            };

            if !should_run {
                log::warn(&format!(
                    "siteMap already exists at {} (use --force or --refresh-days to overwrite)",
                    site_map_path.display()
                ));
                return Ok(());
            }

            let client = http::create_async_client(&config.crawler)?;
            let campuses = run_mapper(config.as_ref(), &locale, &seed, &client).await?;
            fs::save_json(&site_map_path, &campuses)?;

            log::success(
                &locale
                    .messages
                    .mapper_complete
                    .replace("{path}", &site_map_path.display().to_string()),
            );
        }
        Command::Crawl { site_map } => {
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

            // Support switching storage
            process_crawl_with_storage(&cli.storage, Arc::clone(&config), &locale, &campuses)
                .await?;
        }
        Command::Validate => {
            // Validation logic remains the same...
            log::header(&locale.messages.validate_starting);
            if let Err(err) = config.validate() {
                log::error(
                    &locale
                        .messages
                        .validate_failed
                        .replace("{error}", &err.to_string()),
                );
                return Err(err);
            }
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
        }
        Command::Load { from } => match from {
            LoadFrom::New => {
                // Load also needs to respect the storage option
                match cli.storage {
                    StorageMode::S3 => {
                        let storage = S3Storage::from_env().await?;
                        load_and_print(&storage, &locale, &config).await?;
                    }
                    StorageMode::Local => {
                        // Assuming LocalStorage has a default constructor or similar
                        let storage = LocalStorage::new(PathBuf::from("data/storage"));
                        load_and_print(&storage, &locale, &config).await?;
                    }
                }
            }
            LoadFrom::Month(month) => {
                // Explicit unsupported error
                log::error(&format!(
                    "TODO: Loading from month {} is not yet implemented.",
                    month
                ));

                return Err(AppError::config(format!(
                    "Feature not implemented: load --from {month}"
                )));
            }
        },
        Command::Pipeline { skip_map } => {
            // Unify pipeline logic
            // Instead of calling `run_pipeline` (opaque), we execute steps explicitly.
            // 1. Map Phase
            let base = std::env::current_dir()?;
            let site_map_path = config.departments_boards_path(&base);
            let client = http::create_async_client(&config.crawler)?;

            let campuses = if skip_map {
                if !site_map_path.exists() {
                    return Err(AppError::config(format!(
                        "Site map not found at {}. Cannot skip map step.",
                        site_map_path.display()
                    )));
                }
                log::info(
                    &locale
                        .messages
                        .crawler_loading_sitemap
                        .replace("{path}", &site_map_path.display().to_string()),
                );
                Campus::load_all(&site_map_path)?
            } else {
                let generated = run_mapper(config.as_ref(), &locale, &seed, &client).await?;
                fs::save_json(&site_map_path, &generated)?;
                generated
            };

            // 2. Bundle Phase (Executed consistently for both paths)
            // Storage selection logic is handled inside the helper
            match cli.storage {
                StorageMode::S3 => {
                    let storage = S3Storage::from_env().await?;
                    storage
                        .write_config_bundle(config.as_ref(), &seed, &locale, site_map)
                        .await?;
                    run_crawler(Arc::clone(&config), &locale, &storage, &campuses, &client).await?;
                }
                StorageMode::Local => {
                    let storage = LocalStorage::new(PathBuf::from("data/storage"));
                    storage
                        .write_config_bundle(config.as_ref(), &seed, &locale, site_map)
                        .await?;
                    run_crawler(Arc::clone(&config), &locale, &storage, &campuses, &client).await?;
                }
            }
        }
    }

    Ok(())
}

/// Helper to run crawler with the selected storage backend.
/// This avoids code duplication while preserving type safety for the `NoticeStorage` trait.
async fn process_crawl_with_storage(
    mode: &StorageMode,
    config: Arc<Config>,
    locale: &LocaleConfig,
    campuses: &[Campus],
) -> Result<()> {
    let client = http::create_async_client(&config.crawler)?;

    match mode {
        StorageMode::S3 => {
            #[cfg(feature = "s3")]
            {
                let storage = S3Storage::from_env().await?;
                run_crawler(config, locale, &storage, campuses, &client).await
            }
            #[cfg(not(feature = "s3"))]
            {
                return Err(AppError::config(
                    "Built without 's3' feature. Rebuild with: cargo run -F 'cli,s3' ...",
                ));
            }
        }
        StorageMode::Local => {
            // Assuming LocalStorage uses a relative path "data/storage" for dev
            let storage = LocalStorage::new(PathBuf::from("data/storage"));
            run_crawler(config, locale, &storage, campuses, &client).await
        }
    }
}

/// Helper to load and print notices (for Load command)
async fn load_and_print<S: NoticeStorage>(
    storage: &S,
    locale: &LocaleConfig,
    config: &Config,
) -> Result<()> {
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_parses_new() {
        let value: LoadFrom = "new".parse().expect("should parse new");
        assert!(matches!(value, LoadFrom::New));
    }

    #[test]
    fn load_from_parses_month() {
        let value: LoadFrom = "2025-01".parse().expect("should parse month");
        assert!(matches!(value, LoadFrom::Month(ref m) if m == "2025-01"));
    }

    #[test]
    fn load_from_rejects_invalid() {
        assert!("2025-13".parse::<LoadFrom>().is_err());
        assert!("2025-1".parse::<LoadFrom>().is_err());
        assert!("2025-012".parse::<LoadFrom>().is_err());
    }
}
