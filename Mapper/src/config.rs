//! Configuration loading utilities.

use std::path::Path;

use crate::error::{MapperError, Result};
use crate::models::{Config, Seed};
use crate::utils::fs::load_toml;

/// Load configuration from a TOML file
pub fn load_config(path: &Path) -> Result<Config> {
    match load_toml(path) {
        Ok(config) => Ok(config),
        Err(e) => {
            eprintln!("Warning: Failed to load config from {:?}: {}", path, e);
            eprintln!("Using default configuration.");
            Ok(Config::default())
        }
    }
}

/// Load seed data from a TOML file
pub fn load_seed(path: &Path) -> Result<Seed> {
    match load_toml(path) {
        Ok(seed) => Ok(seed),
        Err(e) => {
            eprintln!("Warning: Failed to load seed from {:?}: {}", path, e);
            eprintln!("Using default seed data.");
            Ok(Seed::default())
        }
    }
}

/// Load both config and seed, returning useful error messages
pub fn load_all(base_path: &Path) -> Result<(Config, Seed)> {
    let config_path = base_path.join("data/config.toml");
    let config = load_config(&config_path)?;

    let seed_path = config.seed_path(base_path);
    let seed = load_seed(&seed_path)?;

    // Validate seed has at least some data
    if seed.campuses.is_empty() {
        return Err(MapperError::Config(
            "No campuses defined in seed data".to_string(),
        ));
    }

    if seed.keywords.is_empty() {
        return Err(MapperError::Config(
            "No keywords defined in seed data".to_string(),
        ));
    }

    Ok((config, seed))
}
