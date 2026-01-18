// src/config.rs

//! Configuration loading utilities.

use std::path::Path;

use serde::de::DeserializeOwned;
use tracing::info;

use crate::error::{AppError, Result};
use crate::models::{Config, LocaleConfig, Seed};
use crate::storage::{ByteReader, paths};
use crate::utils::{fs::load_toml, log};

/// Generic config loader backed by a ByteReader (S3, Local, etc.).
pub struct RemoteConfigLoader<R: ByteReader> {
    reader: R,
    prefix: String,
}

impl<R: ByteReader> RemoteConfigLoader<R> {
    pub fn new(reader: R, prefix: impl Into<String>) -> Self {
        Self {
            reader,
            prefix: prefix.into(),
        }
    }

    async fn load_toml<T: DeserializeOwned>(&self, file_name: &str) -> Result<T> {
        let key = paths::config_key(&self.prefix, file_name);
        info!("Loading config file from storage: {}", key);

        let maybe_bytes = self.reader.read_bytes_optional(&key).await?;
        let bytes = maybe_bytes.ok_or_else(|| {
            AppError::config(format!("Config file not found in storage: {}", key))
        })?;

        let s = String::from_utf8(bytes).map_err(|e| {
            AppError::config(format!("Config file {} is not valid UTF-8: {}", key, e))
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

#[cfg(feature = "s3")]
pub type LambdaConfigLoader = RemoteConfigLoader<crate::storage::s3::S3Storage>;

/// Load configuration from a TOML file (local).
pub fn load_config(path: &Path) -> Result<Config> {
    load_toml(path).or_else(|e| {
        log::warn(&format!(
            "Warning: Failed to load config from {path:?}: {e}"
        ));
        log::warn("Using default configuration.");
        Ok(Config::default())
    })
}

/// Load seed data from a TOML file (local).
pub fn load_seed(path: &Path) -> Result<Seed> {
    load_toml(path).or_else(|e| {
        log::warn(&format!("Warning: Failed to load seed from {path:?}: {e}"));
        log::warn("Using default seed data.");
        Ok(Seed::default())
    })
}

/// Load and validate both config and seed data (local).
pub fn load_all(base_path: &Path) -> Result<(Config, Seed)> {
    let config_path = base_path.join("data/config.toml");
    let config = load_config(&config_path)?;

    let seed_path = config.seed_path(base_path);
    let seed = load_seed(&seed_path)?;

    seed.validate()
        .map_err(|e| AppError::config(format!("Invalid seed data: {e}")))?;

    Ok((config, seed))
}
