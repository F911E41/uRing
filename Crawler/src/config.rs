// src/config.rs

//! Configuration loading utilities.
//!
//! This module provides convenience functions for loading configuration
//! and seed data from files.

use std::path::Path;

use serde::de::DeserializeOwned;
use tracing::info;

use crate::storage::s3::S3Storage;

use crate::error::{AppError, Result};
use crate::models::{Config, LocaleConfig, Seed};
use crate::utils::{fs::load_toml, log};

/// Config loader for Lambda environment.
pub struct LambdaConfigLoader {
    storage: S3Storage,
    prefix: String,
}

impl LambdaConfigLoader {
    pub fn new(storage: S3Storage, config_prefix: &str) -> Self {
        Self {
            storage,
            prefix: config_prefix.to_string(),
        }
    }

    async fn load_toml<T: DeserializeOwned>(&self, file_name: &str) -> Result<T> {
        let key = format!("{}/{}", self.prefix, file_name);
        info!("Loading config file from S3: {}", key);
        let maybe_bytes: Option<Vec<u8>> = self.storage.read_bytes_optional(&key).await?;
        let bytes: Vec<u8> = maybe_bytes
            .ok_or_else(|| AppError::Config(format!("Config file not found in S3: {}", key)))?;

        let s = String::from_utf8(bytes).map_err(|e| {
            AppError::Config(format!("Config file {} is not valid UTF-8: {}", key, e))
        })?;
        toml::from_str(&s).map_err(AppError::from)
    }

    pub async fn load_config(&self) -> Result<Config> {
        self.load_toml("config.toml").await
    }

    pub async fn load_locale(&self) -> Result<LocaleConfig> {
        self.load_toml("locale.toml").await
    }

    pub async fn load_seed(&self) -> Result<Seed> {
        self.load_toml("seed.toml").await
    }
}

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
