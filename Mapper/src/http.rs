//! HTTP utilities for fetching web pages.

use reqwest::blocking::Client;
use scraper::Html;
use std::time::Duration;

use crate::config::{REQUEST_TIMEOUT_SECS, USER_AGENT};
use crate::error::Result;

/// Create a configured HTTP client
pub fn create_client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
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
