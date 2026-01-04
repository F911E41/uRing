//! File system utilities.

use std::fs;
use std::path::Path;

use crate::error::Result;

/// Save data to a JSON file with pretty printing
pub fn save_json<T: serde::Serialize>(path: &Path, data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load TOML configuration from a file
pub fn load_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let content = fs::read_to_string(path)?;
    let data: T = toml::from_str(&content)?;
    Ok(data)
}

/// Ensure a directory exists
pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    Ok(())
}
