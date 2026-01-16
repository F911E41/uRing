// src/utils/mod.rs

//! Utility functions and helpers.

pub mod fs;
pub mod http;
pub mod log;
pub mod url;

/// Resolve a potentially relative URL against a base URL.
pub fn resolve_url(base: &::url::Url, href: &str) -> String {
    base.join(href)
        .map(|u: ::url::Url| u.to_string())
        .unwrap_or_else(|_| href.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_url() {
        let base = ::url::Url::parse("https://example.com/path/").unwrap();
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
}
