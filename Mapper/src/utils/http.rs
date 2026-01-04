//! HTTP client utilities.

use std::time::Duration;

use reqwest::blocking::Client;
use scraper::Html;

use crate::error::Result;
use crate::models::HttpConfig;

/// Create a configured HTTP client
pub fn create_client(config: &HttpConfig) -> Result<Client> {
    Ok(Client::builder()
        .user_agent(&config.user_agent)
        .timeout(Duration::from_secs(config.timeout_secs))
        .build()?)
}

/// Fetch a page and return the HTML document
pub fn fetch_page(client: &Client, url: &str) -> Result<Html> {
    let response = client.get(url).send()?;
    let text = response.text()?;
    Ok(Html::parse_document(&text))
}

/// Fetch a page with a custom timeout
pub fn fetch_page_with_timeout(client: &Client, url: &str, timeout_secs: u64) -> Result<Html> {
    let response = client
        .get(url)
        .timeout(Duration::from_secs(timeout_secs))
        .send()?;
    let text = response.text()?;
    Ok(Html::parse_document(&text))
}
