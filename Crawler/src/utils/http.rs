// src/utils/http.rs

//! HTTP client utilities.

use std::time::Duration;

use scraper::Html;

use crate::error::Result;
use crate::models::CrawlerConfig;

// --- Async Functions ---

/// Create a configured asynchronous HTTP client.
pub fn create_async_client(config: &CrawlerConfig) -> Result<reqwest::Client> {
    let client = reqwest::Client::builder()
        .user_agent(&config.user_agent)
        .timeout(Duration::from_secs(config.timeout_secs))
        .build()?;
    Ok(client)
}

/// Fetch a page asynchronously and parse it as HTML.
pub async fn fetch_page_async(client: &reqwest::Client, url: &str) -> Result<Html> {
    let text = client.get(url).send().await?.text().await?;
    Ok(Html::parse_document(&text))
}

// --- Blocking Functions (Deprecated) ---

/// Create a configured blocking HTTP client.
#[deprecated(note = "Use create_async_client instead")]
pub fn create_client(config: &CrawlerConfig) -> Result<reqwest::blocking::Client> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(&config.user_agent)
        .timeout(Duration::from_secs(config.timeout_secs))
        .build()?;
    Ok(client)
}

/// Fetch a page and parse it as HTML.
#[deprecated(note = "Use fetch_page_async instead")]
pub fn fetch_page(client: &reqwest::blocking::Client, url: &str) -> Result<Html> {
    let response = client.get(url).send()?;
    let text = response.text()?;
    Ok(Html::parse_document(&text))
}

/// Fetch a page with a custom timeout.
#[deprecated(note = "Use fetch_page_async instead")]
pub fn fetch_page_with_timeout(
    client: &reqwest::blocking::Client,
    url: &str,
    timeout_secs: u64,
) -> Result<Html> {
    let response = client
        .get(url)
        .timeout(Duration::from_secs(timeout_secs))
        .send()?;
    let text = response.text()?;
    Ok(Html::parse_document(&text))
}
