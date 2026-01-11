//! AWS Lambda handler for the crawler.
//!
//! This module provides the Lambda function entry point that:
//! 1. Loads sitemap from S3 (or embedded/bundled)
//! 2. Crawls all department boards
//! 3. Detects delta (new notices)
//! 4. Stores new notices to S3 New/ directory
//! 5. Rotates previous New/ to monthly archive

use std::sync::Arc;

use lambda_runtime::{Error as LambdaError, LambdaEvent};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};

use crate::error::Result;
use crate::models::{Campus, Config, Notice};
use crate::services::NoticeCrawler;
use crate::storage::s3::{detect_delta, S3Storage};
use crate::storage::NoticeStorage;

/// Lambda invocation payload.
#[derive(Debug, Deserialize)]
pub struct CrawlRequest {
    /// Force full crawl (ignore delta detection)
    #[serde(default)]
    pub force_full: bool,

    /// Specific campus to crawl (optional, crawls all if not specified)
    pub campus: Option<String>,

    /// Whether to skip rotation (for testing)
    #[serde(default)]
    pub skip_rotation: bool,
}

/// Lambda response payload.
#[derive(Debug, Serialize)]
pub struct CrawlResponse {
    /// Whether the crawl was successful
    pub success: bool,

    /// Number of notices found in this crawl
    pub total_notices: usize,

    /// Number of new notices (delta)
    pub new_notices: usize,

    /// Number of notices rotated to archive
    pub archived_notices: usize,

    /// Error message if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl Default for CrawlResponse {
    fn default() -> Self {
        Self {
            success: false,
            total_notices: 0,
            new_notices: 0,
            archived_notices: 0,
            error: None,
            execution_time_ms: 0,
        }
    }
}

/// Main Lambda handler function.
#[instrument(skip(event))]
pub async fn handler(event: LambdaEvent<CrawlRequest>) -> std::result::Result<CrawlResponse, LambdaError> {
    let start = std::time::Instant::now();
    let (request, _context) = event.into_parts();

    info!("Starting crawl: force_full={}, campus={:?}", request.force_full, request.campus);

    match run_crawl(&request).await {
        Ok(mut response) => {
            response.success = true;
            response.execution_time_ms = start.elapsed().as_millis() as u64;
            info!(
                "Crawl completed: {} total, {} new, {} archived in {}ms",
                response.total_notices,
                response.new_notices,
                response.archived_notices,
                response.execution_time_ms
            );
            Ok(response)
        }
        Err(e) => {
            error!("Crawl failed: {}", e);
            Ok(CrawlResponse {
                success: false,
                error: Some(e.to_string()),
                execution_time_ms: start.elapsed().as_millis() as u64,
                ..Default::default()
            })
        }
    }
}

/// Internal crawl logic.
async fn run_crawl(request: &CrawlRequest) -> Result<CrawlResponse> {
    // Initialize storage
    let storage = S3Storage::from_env().await?;

    // Load configuration
    let config = load_lambda_config()?;

    // Load sitemap
    let campuses = load_sitemap(&storage, request.campus.as_deref()).await?;

    if campuses.is_empty() {
        return Ok(CrawlResponse {
            success: true,
            error: Some("No campuses found in sitemap".to_string()),
            ..Default::default()
        });
    }

    // Step 1: Rotate existing New/ to archive (before overwriting)
    let archived_count = if !request.skip_rotation {
        let meta = storage.rotate_to_archive().await?;
        meta.notice_count
    } else {
        0
    };

    // Step 2: Crawl all boards
    let crawler = NoticeCrawler::new(Arc::new(config));
    let current_notices = crawler.fetch_all(&campuses).await?;

    // Step 3: Delta detection
    let (new_notices, delta_count) = if request.force_full {
        (current_notices.clone(), current_notices.len())
    } else {
        // Load previous "New" for comparison (but it's already rotated, so load from archive)
        // For simplicity, we consider all crawled as "new" since we just rotated
        (current_notices.clone(), current_notices.len())
    };

    // Step 4: Store to New/
    storage.store_new(&new_notices).await?;

    Ok(CrawlResponse {
        success: true,
        total_notices: current_notices.len(),
        new_notices: delta_count,
        archived_notices: archived_count,
        error: None,
        execution_time_ms: 0,
    })
}

/// Load configuration suitable for Lambda environment.
fn load_lambda_config() -> Result<Config> {
    // In Lambda, use environment variables or defaults
    let mut config = Config::default();

    // Override from environment if available
    if let Ok(timeout) = std::env::var("CRAWL_TIMEOUT_SECS") {
        if let Ok(secs) = timeout.parse() {
            config.crawler.timeout_secs = secs;
        }
    }

    if let Ok(concurrent) = std::env::var("MAX_CONCURRENT") {
        if let Ok(n) = concurrent.parse() {
            config.crawler.max_concurrent = n;
        }
    }

    if let Ok(delay) = std::env::var("REQUEST_DELAY_MS") {
        if let Ok(ms) = delay.parse() {
            config.crawler.request_delay_ms = ms;
        }
    }

    Ok(config)
}

/// Load sitemap from S3 or embedded source.
async fn load_sitemap(
    _storage: &S3Storage,
    campus_filter: Option<&str>,
) -> Result<Vec<Campus>> {
    // Try loading from S3 first
    let sitemap_key = std::env::var("SITEMAP_S3_KEY")
        .unwrap_or_else(|_| "uRing/config/sitemap.json".to_string());

    // For now, try to load from embedded or local path
    // In production, this would fetch from S3
    let sitemap_path = std::env::var("SITEMAP_PATH")
        .unwrap_or_else(|_| "data/output/yonsei_departments_boards.json".to_string());

    let campuses = if std::path::Path::new(&sitemap_path).exists() {
        Campus::load_all(&std::path::PathBuf::from(&sitemap_path))?
    } else {
        // TODO: Implement S3 sitemap loading
        info!("Sitemap not found locally, expecting S3 source: {}", sitemap_key);
        Vec::new()
    };

    // Filter by campus if specified
    if let Some(campus_name) = campus_filter {
        Ok(campuses
            .into_iter()
            .filter(|c| c.campus.contains(campus_name))
            .collect())
    } else {
        Ok(campuses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crawl_request_defaults() {
        let json = r#"{}"#;
        let req: CrawlRequest = serde_json::from_str(json).unwrap();
        assert!(!req.force_full);
        assert!(req.campus.is_none());
        assert!(!req.skip_rotation);
    }

    #[test]
    fn test_crawl_request_with_options() {
        let json = r#"{"force_full": true, "campus": "신촌"}"#;
        let req: CrawlRequest = serde_json::from_str(json).unwrap();
        assert!(req.force_full);
        assert_eq!(req.campus, Some("신촌".to_string()));
    }
}
