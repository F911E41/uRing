//! Department crawler for Yonsei University.

use regex::Regex;
use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Selector};

use crate::config::CAMPUSES;
use crate::error::Result;
use crate::http::fetch_page;
use crate::models::{Campus, College, Department};

/// Find all homepage URLs in the document and map them by position
fn extract_all_homepage_urls(document: &Html) -> Vec<(usize, String)> {
    let link_selector = Selector::parse("a").unwrap();
    let mut urls = Vec::new();
    let html = document.html();

    for element in document.select(&link_selector) {
        let text: String = element.text().collect();
        if text.contains("홈페이지") {
            if let Some(href) = element.value().attr("href") {
                if href.starts_with("http") && !href.starts_with('#') {
                    // Find position in HTML for ordering
                    if let Some(pos) = html.find(href) {
                        urls.push((pos, href.to_string()));
                    }
                }
            }
        }
    }

    urls.sort_by_key(|(pos, _)| *pos);
    urls
}

/// Extract department info: (name, url) pairs from the main content
fn extract_departments_from_main(
    main_elem: ElementRef,
    document: &Html,
) -> Vec<(String, String, String)> {
    let h1_selector = Selector::parse("h1").unwrap();
    let college_pattern = Regex::new(r"([가-힣]+대학)$").unwrap();

    let mut results: Vec<(String, String, String)> = Vec::new(); // (college, dept_name, url)
    let mut current_college = String::new();

    // Get all homepage URLs
    let homepage_urls = extract_all_homepage_urls(document);
    let mut url_iter = homepage_urls.into_iter().peekable();

    let html = main_elem.html();

    for header in Html::parse_fragment(&html).select(&h1_selector) {
        let mut text = header.text().collect::<String>();

        // Clean up text
        if let Some(idx) = text.find("교수진") {
            text = text[..idx].to_string();
        }
        if let Some(idx) = text.find("홈페이지") {
            text = text[..idx].to_string();
        }
        let text = text.trim().to_string();

        if text.is_empty() {
            continue;
        }

        // Check if this is a college header
        if college_pattern.is_match(&text) {
            current_college = text;
        } else if !current_college.is_empty() && !text.contains("대학") {
            // This is a department - get next URL
            let dept_url = url_iter
                .next()
                .map(|(_, url)| url)
                .unwrap_or_else(|| "NOT_FOUND".to_string());
            results.push((current_college.clone(), text, dept_url));
        }
    }

    results
}

/// Generate a unique department ID
fn generate_department_id(name: &str, url: &str) -> String {
    if url != "NOT_FOUND" {
        let re = Regex::new(r"https?://([^.]+)\.yonsei\.ac\.kr").unwrap();
        if let Some(caps) = re.captures(url) {
            if let Some(subdomain) = caps.get(1) {
                return format!("yonsei_{}", subdomain.as_str().to_lowercase());
            }
        }
    }

    format!("yonsei_{}", name.to_lowercase().replace(' ', "_"))
}

/// Crawl a single campus to extract colleges and departments
pub fn crawl_campus(client: &Client, url: &str, campus_name: &str) -> Result<Campus> {
    let document = fetch_page(client, url)?;

    let mut campus = Campus {
        campus: campus_name.to_string(),
        colleges: Vec::new(),
    };

    let main_selector = Selector::parse("main").unwrap();

    let main = match document.select(&main_selector).next() {
        Some(m) => m,
        None => {
            println!("  Cannot find main content area for {}", campus_name);
            return Ok(campus);
        }
    };

    // Extract all departments with their colleges and URLs
    let dept_info = extract_departments_from_main(main, &document);

    // Group by college
    for (college_name, dept_name, dept_url) in dept_info {
        // Find or create college
        let college_idx = campus
            .colleges
            .iter()
            .position(|c| c.name == college_name)
            .unwrap_or_else(|| {
                campus.colleges.push(College {
                    name: college_name.clone(),
                    departments: Vec::new(),
                });
                campus.colleges.len() - 1
            });

        // Skip duplicate departments
        if campus.colleges[college_idx]
            .departments
            .iter()
            .any(|d| d.name == dept_name)
        {
            continue;
        }

        if dept_url == "NOT_FOUND" {
            println!("  Warning: No homepage URL found for {}", dept_name);
        }

        let dept_id = generate_department_id(&dept_name, &dept_url);
        campus.colleges[college_idx].departments.push(Department {
            id: dept_id,
            name: dept_name,
            url: dept_url,
            boards: Vec::new(),
        });
    }

    Ok(campus)
}

/// Crawl all configured campuses
pub fn crawl_all_campuses(client: &Client) -> Result<Vec<Campus>> {
    let mut campuses = Vec::new();

    for (url, name) in CAMPUSES {
        println!("Crawling {}...", name);
        match crawl_campus(client, url, name) {
            Ok(campus) => {
                let dept_count: usize = campus.colleges.iter().map(|c| c.departments.len()).sum();
                println!("  Found {} departments", dept_count);
                campuses.push(campus);
            }
            Err(e) => {
                println!("  Failed to crawl {}: {}", name, e);
            }
        }
    }

    Ok(campuses)
}
