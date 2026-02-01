//! Utility functions and helpers.

pub mod http;

use url::Url;

/// Resolve a potentially relative URL against a base URL.
pub fn resolve_url(base: &Url, href: &str) -> String {
    base.join(href)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| href.to_string())
}

/// Resolve a URL string against a base URL string.
pub fn resolve(base_url: &str, href: &str) -> Option<String> {
    Url::parse(base_url)
        .ok()
        .map(|base| resolve_url(&base, href))
}

/// Extract the domain from a URL string.
pub fn get_domain(url_str: &str) -> Option<String> {
    Url::parse(url_str)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
}

/// Extract notice ID from a URL (looks for common patterns).
pub fn extract_notice_id(url: &str) -> Option<String> {
    // Common patterns: ?id=123, /notice/123, /view/123, &seq=123
    let patterns = [
        regex::Regex::new(r"[?&](?:id|seq|no|idx|article_seq|articleNo)=(\d+)").ok()?,
        regex::Regex::new(r"/(?:view|notice|article|board)/(\d+)").ok()?,
    ];

    for pattern in &patterns {
        if let Some(caps) = pattern.captures(url) {
            if let Some(id) = caps.get(1) {
                return Some(id.as_str().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_url() {
        let base = Url::parse("https://example.com/path/").unwrap();
        assert_eq!(
            resolve_url(&base, "page.html"),
            "https://example.com/path/page.html"
        );
        assert_eq!(
            resolve_url(&base, "/root.html"),
            "https://example.com/root.html"
        );
        assert_eq!(
            resolve_url(&base, "https://other.com/x"),
            "https://other.com/x"
        );
    }

    #[test]
    fn test_get_domain() {
        assert_eq!(
            get_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            get_domain("https://sub.example.com:8080/path"),
            Some("sub.example.com".to_string())
        );
    }

    #[test]
    fn test_extract_notice_id() {
        assert_eq!(
            extract_notice_id("https://example.com/view?id=123"),
            Some("123".to_string())
        );
        assert_eq!(
            extract_notice_id("https://example.com/notice/456"),
            Some("456".to_string())
        );
    }
}
