//! URL manipulation utilities.

/// Resolve a potentially relative URL against a base URL
pub fn resolve(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }

    if href.starts_with('/') {
        // Absolute path - extract base domain
        if let Some(idx) = base.find("://") {
            let after_scheme = &base[idx + 3..];
            if let Some(slash_idx) = after_scheme.find('/') {
                let domain = &base[..idx + 3 + slash_idx];
                return format!("{}{}", domain, href);
            }
        }
        return format!("{}{}", base.trim_end_matches('/'), href);
    }

    // Relative path
    let base_without_file = if base.ends_with('/') {
        base.to_string()
    } else {
        match base.rfind('/') {
            Some(idx) => base[..=idx].to_string(),
            None => format!("{}/", base),
        }
    };

    format!("{}{}", base_without_file, href)
}

/// Extract domain from URL
pub fn get_domain(url: &str) -> Option<String> {
    if let Some(idx) = url.find("://") {
        let after_scheme = &url[idx + 3..];
        let domain = after_scheme.split('/').next()?;
        return Some(domain.to_lowercase());
    }
    None
}
