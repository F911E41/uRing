//! Board discovery for department websites.

use regex::Regex;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};

use crate::config::KEYWORD_MAP;
use crate::http::fetch_page_with_timeout;
use crate::models::{Board, BoardDiscoveryResult, ManualReviewItem};
use crate::selectors::{detect_cms_selectors, CmsSelectors};

/// Check if a link is likely a valid board link
fn is_valid_board_link(text: &str, href: &str) -> bool {
    // Blacklist patterns that indicate article views, not board listings
    let blacklist = [
        "articleNo",
        "article_no",
        "mode=view",
        "seq",
        "view.do",
        "board_seq",
    ];
    if blacklist.iter().any(|word| href.contains(word)) {
        return false;
    }

    // Long text is likely a notice title, not a board name
    if text.chars().count() > 20 {
        return false;
    }

    true
}

/// Try to find and fetch the sitemap page
fn find_sitemap(client: &Client, document: &Html, base_url: &str) -> Option<Html> {
    let link_selector = Selector::parse("a").unwrap();
    let sitemap_pattern = Regex::new(r"(?i)사이트맵|sitemap").unwrap();

    for element in document.select(&link_selector) {
        let text: String = element.text().collect();
        if sitemap_pattern.is_match(&text) {
            if let Some(href) = element.value().attr("href") {
                let sitemap_url = resolve_url(base_url, href);
                if let Ok(sitemap_doc) = fetch_page_with_timeout(client, &sitemap_url, 5) {
                    println!("    Found sitemap: {}", sitemap_url);
                    return Some(sitemap_doc);
                }
            }
        }
    }

    None
}

/// Resolve a potentially relative URL against a base URL
fn resolve_url(base: &str, href: &str) -> String {
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
fn get_domain(url: &str) -> Option<String> {
    if let Some(idx) = url.find("://") {
        let after_scheme = &url[idx + 3..];
        let domain = after_scheme.split('/').next()?;
        return Some(domain.to_lowercase());
    }
    None
}

/// Extract boards from an HTML document
fn extract_boards_from_document(
    document: &Html,
    base_url: &str,
    default_selectors: &Option<CmsSelectors>,
) -> Vec<Board> {
    let mut boards = Vec::new();
    let mut seen_urls = HashSet::new();
    let mut id_counts: HashMap<String, usize> = HashMap::new();

    let base_domain = get_domain(base_url);
    let link_selector = Selector::parse("a[href]").unwrap();

    for element in document.select(&link_selector) {
        let text: String = element.text().collect::<String>().trim().to_string();
        let href = match element.value().attr("href") {
            Some(h) => h,
            None => continue,
        };

        if !is_valid_board_link(&text, href) {
            continue;
        }

        let full_url = resolve_url(base_url, href);

        // Skip invalid URLs
        if seen_urls.contains(&full_url) || href.contains("javascript") || href == "#" {
            continue;
        }

        // Skip external links
        if let (Some(base_dom), Some(link_dom)) = (&base_domain, get_domain(&full_url)) {
            if base_dom != &link_dom {
                continue;
            }
        }

        // Match against keywords
        for (keyword, meta) in KEYWORD_MAP {
            if !text.contains(keyword) {
                continue;
            }

            // Try to detect CMS selectors
            let selectors =
                detect_cms_selectors(document, &full_url).or_else(|| default_selectors.clone());

            let selectors = match selectors {
                Some(s) => s,
                None => continue,
            };

            // Generate unique ID
            let base_id = meta.id.to_string();
            let count = id_counts.entry(base_id.clone()).or_insert(0);
            *count += 1;

            let final_id = if *count > 1 {
                format!("{}_{}", base_id, count)
            } else {
                base_id
            };

            let board_name = if text.is_empty() {
                meta.name.to_string()
            } else {
                text.clone()
            };

            boards.push(Board {
                id: final_id,
                name: board_name,
                url: full_url.clone(),
                selectors,
            });

            seen_urls.insert(full_url.clone());
            break;
        }
    }

    boards
}

/// Discover useful boards from a department homepage
pub fn discover_boards(
    client: &Client,
    campus: &str,
    dept_name: &str,
    dept_url: &str,
) -> BoardDiscoveryResult {
    let mut result = BoardDiscoveryResult::default();

    // Validate URL
    if dept_url == "NOT_FOUND" || !dept_url.starts_with("http") {
        result.manual_review = Some(ManualReviewItem {
            campus: campus.to_string(),
            name: dept_name.to_string(),
            url: dept_url.to_string(),
            reason: "Homepage URL is invalid".to_string(),
        });
        return result;
    }

    // Fetch department homepage
    let document = match fetch_page_with_timeout(client, dept_url, 7) {
        Ok(doc) => doc,
        Err(e) => {
            result.manual_review = Some(ManualReviewItem {
                campus: campus.to_string(),
                name: dept_name.to_string(),
                url: dept_url.to_string(),
                reason: format!("Failed to fetch homepage: {}", e),
            });
            return result;
        }
    };

    println!("    Accessed: {}", dept_url);

    // Detect default CMS selectors
    let default_selectors = detect_cms_selectors(&document, dept_url);

    // Try sitemap first
    if let Some(sitemap_doc) = find_sitemap(client, &document, dept_url) {
        let boards = extract_boards_from_document(&sitemap_doc, dept_url, &default_selectors);
        if !boards.is_empty() {
            result.boards = boards;
            return result;
        }
        println!("    Sitemap yielded no results, falling back to homepage");
    }

    // Fall back to homepage
    result.boards = extract_boards_from_document(&document, dept_url, &default_selectors);

    result
}
