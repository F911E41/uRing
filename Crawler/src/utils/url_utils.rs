// src/utils/url_utils.rs

/// Resolve a potentially relative URL against a base URL
pub fn resolve_url(base: &url::Url, href: &str) -> String {
    match base.join(href) {
        Ok(url) => url.to_string(),
        Err(_) => href.to_string(),
    }
}
