// src/utils/http.rs

//! HTTP client utilities.

use std::time::Duration;

use reqwest::{StatusCode, header};
use scraper::Html;

use crate::error::{AppError, Result};
use crate::models::CrawlerConfig;

/// Create a configured asynchronous HTTP client.
pub fn create_async_client(config: &CrawlerConfig) -> Result<reqwest::Client> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        ),
    );
    headers.insert(
        header::ACCEPT_LANGUAGE,
        header::HeaderValue::from_static("ko-KR,ko;q=0.9,en-US;q=0.8,en;q=0.7"),
    );

    let client = reqwest::Client::builder()
        .user_agent(&config.user_agent)
        .default_headers(headers)
        .timeout(Duration::from_secs(config.timeout_secs))
        .connect_timeout(Duration::from_secs(config.timeout_secs.min(10)))
        .pool_idle_timeout(Duration::from_secs(60))
        .tcp_keepalive(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    Ok(client)
}

/// Fetch a page asynchronously and parse it as HTML.
pub async fn fetch_page_async(client: &reqwest::Client, url: &str) -> Result<Html> {
    let resp = client.get(url).send().await?;

    // Process http response
    let status = resp.status();
    if status == StatusCode::NOT_MODIFIED {
        return Err(AppError::UpstreamNotModified {
            url: url.to_string(),
        }
        .into());
    }

    if !status.is_success() {
        return Err(AppError::UpstreamHttp {
            url: url.to_string(),
            status: status.as_u16(),
        }
        .into());
    }

    // Check Content-Type (prevent non-HTML responses)
    if let Some(ct) = resp.headers().get(header::CONTENT_TYPE) {
        let ct = ct.to_str().unwrap_or("");
        if !ct.contains("text/html") && !ct.contains("application/xhtml+xml") {
            return Err(AppError::UpstreamUnexpectedContentType {
                url: url.to_string(),
                content_type: ct.to_string(),
            }
            .into());
        }
    }

    // Size limit (operational stability) - consider moving to config if needed
    // reqwest reads the full body by default, so read as text first
    // Check content-length to prevent large responses (error pages/file downloads).
    if let Some(len) = resp.content_length() {
        let max = 2_000_000u64; // 2MB For example
        if len > max {
            return Err(AppError::UpstreamBodyTooLarge {
                url: url.to_string(),
                bytes: len,
                max_bytes: max,
            }
            .into());
        }
    }

    let text = resp.text().await?;
    Ok(Html::parse_document(&text))
}
