// src/services/departments.rs

//! Department crawler service.
//!
//! Crawls campus pages to discover departments and their homepage URLs.

use regex::Regex;
use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Selector};

use crate::error::Result;
use crate::models::{Campus, CampusInfo, College, Department};
use crate::utils::{http::fetch_page, log};

/// Service for crawling campus department information.
pub struct DepartmentCrawler<'a> {
    client: &'a Client,
}

impl<'a> DepartmentCrawler<'a> {
    /// Create a new department crawler.
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Crawl all campuses and return their departments.
    pub fn crawl_all(&self, campuses: &[CampusInfo]) -> Result<Vec<Campus>> {
        campuses
            .iter()
            .filter_map(|info| {
                log::info(&format!("Crawling {}...", info.name));
                match self.crawl_campus(info) {
                    Ok(campus) => {
                        let count = campus.department_count();
                        log::info(&format!("  Found {count} departments"));
                        Some(campus)
                    }
                    Err(e) => {
                        log::error(&format!("  Failed to crawl {}: {e}", info.name));
                        None
                    }
                }
            })
            .collect::<Vec<_>>()
            .pipe(Ok)
    }

    /// Crawl a single campus.
    fn crawl_campus(&self, info: &CampusInfo) -> Result<Campus> {
        let document = fetch_page(self.client, &info.url)?;

        let mut campus = Campus {
            campus: info.name.clone(),
            colleges: Vec::new(),
            departments: Vec::new(),
        };

        let Some(main_elem) = self.find_main_content(&document) else {
            log::error(&format!(
                "  Cannot find main content area for {}",
                info.name
            ));

            return Ok(campus);
        };

        // Extract departments and group by college
        let dept_info = self.extract_departments_from_main(main_elem, &document);
        self.group_into_colleges(&mut campus, dept_info);

        Ok(campus)
    }

    fn find_main_content<'b>(&self, document: &'b Html) -> Option<ElementRef<'b>> {
        let main_selector = Selector::parse("main").ok()?;
        document.select(&main_selector).next()
    }

    fn group_into_colleges(&self, campus: &mut Campus, dept_info: Vec<(String, String, String)>) {
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

            // Skip duplicates
            if campus.colleges[college_idx]
                .departments
                .iter()
                .any(|d| d.name == dept_name)
            {
                continue;
            }

            if dept_url == "NOT_FOUND" {
                log::warn(&format!("  Warning: No homepage URL found for {dept_name}"));
            }

            let dept_id = Self::generate_department_id(&dept_name, &dept_url);
            campus.colleges[college_idx].departments.push(Department {
                id: dept_id,
                name: dept_name,
                url: dept_url,
                boards: Vec::new(),
            });
        }
    }

    /// Extract departments from main element.
    fn extract_departments_from_main(
        &self,
        main_elem: ElementRef,
        document: &Html,
    ) -> Vec<(String, String, String)> {
        let Ok(h1_selector) = Selector::parse("h1") else {
            return Vec::new();
        };

        let college_pattern = Regex::new(r"([가-힣]+대학)$").unwrap_or_else(|_| {
            // Fallback pattern that won't match anything
            Regex::new(r"^$").unwrap()
        });

        let mut results: Vec<(String, String, String)> = Vec::new();
        let mut current_college = String::new();

        // Get all homepage URLs
        let homepage_urls = self.extract_all_homepage_urls(document);
        let mut url_iter = homepage_urls.into_iter().peekable();

        let html = main_elem.html();
        let fragment = Html::parse_fragment(&html);

        for header in fragment.select(&h1_selector) {
            let text = self.clean_header_text(header);
            if text.is_empty() {
                continue;
            }

            if college_pattern.is_match(&text) {
                current_college = text;
            } else if !current_college.is_empty() && !text.contains("대학") {
                let dept_url = url_iter
                    .next()
                    .map(|(_, url)| url)
                    .unwrap_or_else(|| "NOT_FOUND".to_string());
                results.push((current_college.clone(), text, dept_url));
            }
        }

        results
    }

    fn clean_header_text(&self, header: ElementRef) -> String {
        let mut text: String = header.text().collect();

        // Remove common suffixes
        for suffix in &["교수진", "홈페이지"] {
            if let Some(idx) = text.find(suffix) {
                text = text[..idx].to_string();
            }
        }

        text.trim().to_string()
    }

    /// Find all homepage URLs in the document.
    fn extract_all_homepage_urls(&self, document: &Html) -> Vec<(usize, String)> {
        let Ok(link_selector) = Selector::parse("a") else {
            return Vec::new();
        };

        let html = document.html();
        let mut urls: Vec<(usize, String)> = document
            .select(&link_selector)
            .filter_map(|element| {
                let text: String = element.text().collect();
                if !text.contains("홈페이지") {
                    return None;
                }

                let href = element.value().attr("href")?;
                if !href.starts_with("http") || href.starts_with('#') {
                    return None;
                }

                let pos = html.find(href)?;
                Some((pos, href.to_string()))
            })
            .collect();

        urls.sort_by_key(|(pos, _)| *pos);
        urls
    }

    /// Generate a unique department ID from name or URL.
    fn generate_department_id(name: &str, url: &str) -> String {
        if url != "NOT_FOUND" {
            if let Ok(re) = Regex::new(r"https?://([^.]+)\.yonsei\.ac\.kr") {
                if let Some(caps) = re.captures(url) {
                    if let Some(subdomain) = caps.get(1) {
                        return format!("yonsei_{}", subdomain.as_str().to_lowercase());
                    }
                }
            }
        }

        format!("yonsei_{}", name.to_lowercase().replace(' ', "_"))
    }
}

/// Extension trait for pipe operations.
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}
