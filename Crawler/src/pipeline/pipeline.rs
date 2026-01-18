// src/pipeline/pipeline.rs

use std::sync::Arc;

use reqwest::Client;

use super::crawl::run_crawler;
use super::map::run_mapper;

use crate::error::Result;
use crate::models::{Config, LocaleConfig, Seed};
use crate::storage::NoticeStorage;
use crate::utils::log;

/// Run the full pipeline.
pub async fn run_pipeline(
    config: Arc<Config>,
    locale: &LocaleConfig,
    seed: &Seed,
    storage: &dyn NoticeStorage,
    client: &Client,
) -> Result<()> {
    log::header(&locale.messages.pipeline_starting);

    // Step 1: Discover departments and boards
    log::step(1, 3, "Map - Discovering departments and boards");
    let campuses = run_mapper(config.as_ref(), locale, seed, client).await?;

    // Step 2: Persist config and site map for this run
    log::step(2, 3, "Config - Persisting config and site map");
    storage
        .write_config_bundle(config.as_ref(), seed, &locale, &campuses)
        .await?;

    // Step 3: Crawl notices from the discovered boards
    log::step(3, 3, "Crawl - Fetching notices");
    run_crawler(Arc::clone(&config), locale, storage, &campuses, client).await?;

    log::success(&locale.messages.pipeline_complete);

    Ok(())
}
