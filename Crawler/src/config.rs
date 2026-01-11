// src/config.rs

//! Configuration loading utilities.
//!
//! This module provides convenience functions for loading configuration
//! and seed data from files.

use std::path::Path;

use crate::error::{AppError, Result};
use crate::models::{Config, Seed};
use crate::utils::{fs::load_toml, log};

/// Load configuration from a TOML file.
///
/// Falls back to defaults if loading fails.
pub fn load_config(path: &Path) -> Result<Config> {
    load_toml(path).or_else(|e| {
        log::warn(&format!(
            "Warning: Failed to load config from {path:?}: {e}"
        ));
        log::warn("Using default configuration.");
        Ok(Config::default())
    })
}

/// Load seed data from a TOML file.
///
/// Falls back to defaults if loading fails.
pub fn load_seed(path: &Path) -> Result<Seed> {
    load_toml(path).or_else(|e| {
        log::warn(&format!("Warning: Failed to load seed from {path:?}: {e}"));
        log::warn("Using default seed data.");
        Ok(Seed::default())
    })
}

/// Load and validate both config and seed data.
pub fn load_all(base_path: &Path) -> Result<(Config, Seed)> {
    let config_path = base_path.join("data/config.toml");
    let config = load_config(&config_path)?;

    let seed_path = config.seed_path(base_path);
    let seed = load_seed(&seed_path)?;

    // Validate seed data
    seed.validate()
        .map_err(|e| AppError::config(format!("Invalid seed data: {e}")))?;

    Ok((config, seed))
}
