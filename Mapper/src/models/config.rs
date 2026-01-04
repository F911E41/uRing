//! Configuration model structures.

use serde::Deserialize;
use std::path::PathBuf;

/// Root configuration structure
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub http: HttpConfig,
    pub paths: PathsConfig,
    pub discovery: DiscoveryConfig,
    pub logging: LoggingConfig,
}

/// HTTP client settings
#[derive(Debug, Deserialize, Clone)]
pub struct HttpConfig {
    pub user_agent: String,
    pub timeout_secs: u64,
    pub sitemap_timeout_secs: u64,
}

/// File path configurations
#[derive(Debug, Deserialize, Clone)]
pub struct PathsConfig {
    pub seed: String,
    pub output_dir: String,
    pub departments_file: String,
    pub departments_boards_file: String,
    pub manual_review_file: String,
}

/// Board discovery settings
#[derive(Debug, Deserialize, Clone)]
pub struct DiscoveryConfig {
    pub max_board_name_length: usize,
    pub blacklist_patterns: Vec<String>,
}

/// Logging configurations
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct LoggingConfig {
    pub level: String,
    pub show_progress: bool,
}

impl Config {
    /// Get the full path to the output directory
    pub fn output_dir(&self, base: &PathBuf) -> PathBuf {
        base.join(&self.paths.output_dir)
    }

    /// Get the full path to departments file
    pub fn departments_path(&self, base: &PathBuf) -> PathBuf {
        self.output_dir(base).join(&self.paths.departments_file)
    }

    /// Get the full path to departments with boards file
    pub fn departments_boards_path(&self, base: &PathBuf) -> PathBuf {
        self.output_dir(base)
            .join(&self.paths.departments_boards_file)
    }

    /// Get the full path to manual review file
    pub fn manual_review_path(&self, base: &PathBuf) -> PathBuf {
        self.output_dir(base).join(&self.paths.manual_review_file)
    }

    /// Get the full path to seed file
    pub fn seed_path(&self, base: &std::path::Path) -> PathBuf {
        base.join(&self.paths.seed)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            http: HttpConfig {
                user_agent: "Mozilla/5.0 (compatible; uRing Mapper/0.1)".to_string(),
                timeout_secs: 10,
                sitemap_timeout_secs: 5,
            },
            paths: PathsConfig {
                seed: "data/seed.toml".to_string(),
                output_dir: "data/output".to_string(),
                departments_file: "yonsei_departments.json".to_string(),
                departments_boards_file: "yonsei_departments_boards.json".to_string(),
                manual_review_file: "manual_review_needed.json".to_string(),
            },
            discovery: DiscoveryConfig {
                max_board_name_length: 20,
                blacklist_patterns: vec![
                    "articleNo".to_string(),
                    "article_no".to_string(),
                    "mode=view".to_string(),
                    "seq".to_string(),
                    "view.do".to_string(),
                    "board_seq".to_string(),
                ],
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                show_progress: true,
            },
        }
    }
}
