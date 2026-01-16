// src/pipeline/pipeline.rs

use super::crawl::run_crawler;
use super::map::run_mapper;

use crate::error::Result;
use crate::models::{Config, LocaleConfig, Seed};
use crate::storage::NoticeStorage;
use crate::utils::{http, log};

/// Run the full pipeline.
pub async fn run_pipeline(
    config: &Config,
    locale: &LocaleConfig,
    seed: &Seed,
    storage: &dyn NoticeStorage,
) -> Result<()> {
    log::header(&locale.messages.pipeline_starting);

    let client = http::create_async_client(&config.crawler)?;

    // Step 1: Discover departments and boards
    log::step(1, 3, "Map - Discovering departments and boards");
    let campuses = run_mapper(config, locale, seed, &client).await?;

    // Step 2: Persist config and site map for this run
    log::step(2, 3, "Config - Persisting config and site map");
    storage.write_config_bundle(config, seed, &campuses).await?;

    // Step 3: Crawl notices from the discovered boards
    log::step(3, 3, "Crawl - Fetching notices");
    run_crawler(config, locale, storage, &campuses).await?;

    log::success(&locale.messages.pipeline_complete);

    Ok(())
}
