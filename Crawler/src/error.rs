//! Unified error handling for the crawler application.
//!
//! This module defines the `AppError` enum which encapsulates
//! various error types that can occur throughout the application,
//! including I/O errors, HTTP errors, parsing errors, and domain-specific errors.

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

    /// LocalStorage error
    #[error("Local storage error: {0}")]
    LocalStorage(String),

    /// Upstream returned non-success HTTP status
    #[error("Upstream HTTP {status} for {url}")]
    UpstreamHttp { url: String, status: u16 },

    /// Upstream returned 304 Not Modified
    #[error("Upstream not modified for {url}")]
    UpstreamNotModified { url: String },

    /// Upstream returned unexpected content-type
    #[error("Upstream unexpected content-type for {url}: {content_type}")]
    UpstreamUnexpectedContentType { url: String, content_type: String },

    /// Upstream body too large
    #[error("Upstream body too large for {url}: {bytes} > {max_bytes}")]
    UpstreamBodyTooLarge {
        url: String,
        bytes: u64,
        max_bytes: u64,
    },

    /// Circuit breaker triggered - data drop threshold exceeded
    #[error(
        "Circuit breaker triggered: {current_count} notices vs {previous_count} previous ({drop_percent:.1}% drop > {threshold_percent}% threshold)"
    )]
    CircuitBreakerTriggered {
        current_count: usize,
        previous_count: usize,
        drop_percent: f64,
        threshold_percent: u8,
    },

    /// Empty crawl result
    #[error("Empty crawl result - no notices fetched")]
    EmptyCrawlResult,
}

/// Helper methods for AppError
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

    /// Check retriable error based on HTTP status code.
    pub fn is_retryable(&self) -> bool {
        match self {
            AppError::Http(e) => e.is_timeout() || e.is_connect() || e.is_request(),
            AppError::UpstreamHttp { status, .. } => {
                // 5xx, 429 are retryable
                (500..600).contains(status) || *status == 429
            }
            _ => false,
        }
    }
}

// Backward compatibility type aliases
#[allow(dead_code)]
pub type CrawlerError = AppError;
#[allow(dead_code)]
pub type MapperError = AppError;
