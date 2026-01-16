// src/utils/url.rs

//! URL manipulation utilities.

/// Resolve a potentially relative URL against a base URL.
///
/// # Examples
/// ```
/// use crawler::utils::url::resolve;
///
/// assert_eq!(
///     resolve("https://example.com/path/", "page.html"),
///     "https://example.com/path/page.html"
/// );
/// ```
pub fn resolve(base: &str, href: &str) -> String {
    // Already absolute
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }

    // Absolute path - combine with base domain
    if href.starts_with('/') {
        return resolve_absolute_path(base, href);
    }

    // Relative path - combine with base directory
    resolve_relative_path(base, href)
}

fn resolve_absolute_path(base: &str, href: &str) -> String {
    if let Some(scheme_end) = base.find("://") {
        let after_scheme = &base[scheme_end + 3..];
        if let Some(slash_idx) = after_scheme.find('/') {
            let domain = &base[..scheme_end + 3 + slash_idx];
            return format!("{domain}{href}");
        }
    }
    format!("{}{}", base.trim_end_matches('/'), href)
}

fn resolve_relative_path(base: &str, href: &str) -> String {
    let base_dir = if base.ends_with('/') {
        base.to_string()
    } else {
        match base.rfind('/') {
            Some(idx) => base[..=idx].to_string(),
            None => format!("{base}/"),
        }
    };

    format!("{base_dir}{href}")
}

/// Extract domain from a URL.
///
/// # Examples
/// ```
/// use crawler::utils::url::get_domain;
///
/// assert_eq!(
///     get_domain("https://example.com/path"),
///     Some("example.com".to_string())
/// );
/// ```
pub fn get_domain(url: &str) -> Option<String> {
    let scheme_end = url.find("://")?;
    let after_scheme = &url[scheme_end + 3..];
    let domain = after_scheme.split('/').next()?;
    Some(domain.to_lowercase())
}

/// Extract a stable notice identifier from a URL.
pub fn extract_notice_id(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let mut fallback_keyed: Option<String> = None;
    let mut fallback_numeric: Option<String> = None;

    for (key, value) in parsed.query_pairs() {
        if value.is_empty() {
            continue;
        }

        let key_lower = key.to_lowercase();
        let value_string = value.to_string();

        if matches!(
            key_lower.as_str(),
            "articleno"
                | "article_no"
                | "articleid"
                | "article_id"
                | "board_seq"
                | "notice_id"
                | "noticeid"
                | "seq"
                | "no"
                | "id"
        ) {
            return Some(value_string);
        }

        if fallback_keyed.is_none()
            && (key_lower.contains("id")
                || key_lower.contains("no")
                || key_lower.contains("seq")
                || key_lower.contains("article"))
        {
            fallback_keyed = Some(value_string.clone());
        }

        if fallback_numeric.is_none() && value_string.chars().all(|c| c.is_ascii_digit()) {
            fallback_numeric = Some(value_string);
        }
    }

    if let Some(value) = fallback_keyed {
        return Some(value);
    }
    if let Some(value) = fallback_numeric {
        return Some(value);
    }

    if let Some(last) = parsed.path_segments().and_then(|segments| segments.last()) {
        let digits: String = last.chars().filter(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return Some(digits);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_absolute_url() {
        assert_eq!(
            resolve("https://example.com/path/", "https://other.com/page"),
            "https://other.com/page"
        );
    }

    #[test]
    fn test_resolve_absolute_path() {
        assert_eq!(
            resolve("https://example.com/path/", "/root.html"),
            "https://example.com/root.html"
        );
    }

    #[test]
    fn test_resolve_relative_path() {
        assert_eq!(
            resolve("https://example.com/path/", "page.html"),
            "https://example.com/path/page.html"
        );
    }

    #[test]
    fn test_resolve_relative_from_file() {
        assert_eq!(
            resolve("https://example.com/path/index.html", "other.html"),
            "https://example.com/path/other.html"
        );
    }

    #[test]
    fn test_get_domain() {
        assert_eq!(
            get_domain("https://Example.COM/path"),
            Some("example.com".to_string())
        );
        assert_eq!(get_domain("invalid-url"), None);
    }

    #[test]
    fn test_extract_notice_id_query_key() {
        let url = "https://example.com/view?articleNo=1234&mode=view";
        assert_eq!(extract_notice_id(url), Some("1234".to_string()));
    }

    #[test]
    fn test_extract_notice_id_query_fallback() {
        let url = "https://example.com/view?seq=888";
        assert_eq!(extract_notice_id(url), Some("888".to_string()));
    }

    #[test]
    fn test_extract_notice_id_path_digits() {
        let url = "https://example.com/notice/9999";
        assert_eq!(extract_notice_id(url), Some("9999".to_string()));
    }
}
