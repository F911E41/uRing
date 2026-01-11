// src/pipeline/pipeline.rs

use std::path::PathBuf;

use crate::error::Result;
use crate::models::{Config, LocaleConfig};
use crate::utils::log;

use super::archive::run_archive;
use super::crawl::run_crawler;
use super::map::run_mapper;

/// Run the full pipeline.
pub async fn run_pipeline(
    config: &Config,
    locale: &LocaleConfig,
    base_path: &PathBuf,
    skip_map: bool,
) -> Result<()> {
    log::header(&locale.messages.pipeline_starting);

    let total_steps = if skip_map { 2 } else { 3 };
    let mut current_step = 1;

    if !skip_map {
        log::step(
            current_step,
            total_steps,
            "Map - Discovering departments and boards",
        );
        run_mapper(config, locale, base_path).await?;
        current_step += 1;
    }

    log::step(current_step, total_steps, "Crawl - Fetching notices");
    run_crawler(config, locale, base_path).await?;
    current_step += 1;

    log::step(current_step, total_steps, "Archive - Storing notices");
    run_archive(config, locale).await?;

    log::success(&locale.messages.pipeline_complete);

    Ok(())
}
