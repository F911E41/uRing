// src/error.rs

//! Unified error handling for the crawler application.

use std::fmt;

use thiserror::Error;

/// Result type alias for crawler operations.
pub type Result<T> = std::result::Result<T, AppError>;

/// Unified application error type.
#[derive(Error, Debug)]
pub enum AppError {
    /// AWS S3 error
    #[error("S3 error: {0}")]
    S3(String),

    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// TOML parsing failed
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    /// TOML serialization failed
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// URL parsing failed
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    /// CSS selector parsing failed
    #[error("Invalid selector '{selector}': {message}")]
    Selector { selector: String, message: String },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Data validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Discovery/mapping error
    #[error("Discovery error: {0}")]
    Discovery(String),

    /// Crawling error
    #[error("Crawl error for {context}: {message}")]
    Crawl { context: String, message: String },
}

impl AppError {
    /// Create a selector parsing error.
    pub fn selector(selector: impl Into<String>, message: impl fmt::Display) -> Self {
        Self::Selector {
            selector: selector.into(),
            message: message.to_string(),
        }
    }

    /// Create a configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    /// Create a discovery error.
    pub fn discovery(message: impl Into<String>) -> Self {
        Self::Discovery(message.into())
    }

    /// Create a crawl error with context.
    pub fn crawl(context: impl Into<String>, message: impl fmt::Display) -> Self {
        Self::Crawl {
            context: context.into(),
            message: message.to_string(),
        }
    }
}

// Backward compatibility type aliases
#[allow(dead_code)]
pub type CrawlerError = AppError;
#[allow(dead_code)]
pub type MapperError = AppError;
