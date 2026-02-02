//! Department crawler service.
//!
//! Crawls campus pages to discover departments and their homepage URLs.

use futures::stream::{self, StreamExt, TryStreamExt};
use regex::Regex;
use reqwest::Client;
use scraper::{ElementRef, Html, Selector};

use crate::error::Result;
use crate::models::{Campus, CampusInfo, College, Department};
use crate::utils::http::fetch_page_async;

/// Service for crawling campus department information.
pub struct DepartmentCrawler<'a> {
    client: &'a Client,
}

/// Implementation of DepartmentCrawler
impl<'a> DepartmentCrawler<'a> {
    /// Create a new department crawler.
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Crawl all campuses and return their departments.
    pub async fn crawl_all(&self, campuses: &[CampusInfo]) -> Result<Vec<Campus>> {
        stream::iter(campuses)
            .map(|info| self.crawl_campus(info))
            .buffer_unordered(5) // Concurrently crawl up to 5 campuses
            .try_collect()
            .await
    }

    /// Crawl a single campus.
    async fn crawl_campus(&self, info: &CampusInfo) -> Result<Campus> {
        log::info!("Crawling {}...", info.name);
        let document = fetch_page_async(self.client, &info.url).await?;

        let mut campus = Campus {
            campus: info.name.clone(),
            colleges: Vec::new(),
            departments: Vec::new(),
        };

        let Some(main_elem) = self.find_main_content(&document) else {
            log::error!("Cannot find main content area for {}", info.name);
            return Ok(campus);
        };

        // Extract departments and group by college
        let dept_info = self.extract_departments_from_main(main_elem, &document);
        self.group_into_colleges(&mut campus, dept_info);

        let count = campus.department_count();
        log::info!("Found {} departments in {}", count, info.name);

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
                log::warn!("No homepage URL found for {}", dept_name);
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
        _document: &Html,
    ) -> Vec<(String, String, String)> {
        // Use composite selector to match h1 and a tags in document order
        let Ok(selector) = Selector::parse("h1, a") else {
            return Vec::new();
        };

        // Pattern to match college names ending with "대학"
        // Allow spaces between Korean characters (e.g., "소프트웨어디지털 헬스케어융합대학")
        let college_pattern = Regex::new(r"^([가-힣A-Za-z]+(?:\s*[가-힣A-Za-z]+)*대학)\s*$").unwrap();
        
        // Pattern to match "대학명 학과명" format (e.g., "소프트웨어디지털헬스케어융합대학 소프트웨어학부")
        let college_dept_pattern = Regex::new(r"^([가-힣A-Za-z]+(?:\s*[가-힣A-Za-z]+)*대학)\s+(.+)$").unwrap();
        
        let mut results: Vec<(String, String, String)> = Vec::new();

        let mut current_college = String::new();
        let mut pending_dept: Option<String> = None;

        // Iterate over descendants of the main element
        for element in main_elem.select(&selector) {
            let tag = element.value().name();

            if tag == "h1" {
                // If there was a pending department without a URL, mark it as NOT_FOUND
                if let Some(dept_name) = pending_dept.take() {
                    results.push((current_college.clone(), dept_name, "NOT_FOUND".to_string()));
                }

                let text = self.clean_header_text(element);
                if text.is_empty() {
                    continue;
                }

                // First, check if text is "대학명 학과명" format
                if let Some(caps) = college_dept_pattern.captures(&text) {
                    let college_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    let dept_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                    
                    // Normalize college name by removing extra spaces
                    let normalized_college = college_name.split_whitespace().collect::<Vec<_>>().join("");
                    
                    // Update current college if different
                    if current_college != normalized_college {
                        current_college = normalized_college;
                    }
                    
                    // Set pending department
                    pending_dept = Some(dept_name.trim().to_string());
                } else if college_pattern.is_match(&text) {
                    // Just a college name without department
                    // Normalize college name by removing extra spaces
                    current_college = text.split_whitespace().collect::<Vec<_>>().join("");
                } else if !current_college.is_empty() {
                    // Simple department name (without college prefix)
                    pending_dept = Some(text);
                }
            } else if tag == "a" {
                // Only interested in links if we have a pending department
                if pending_dept.is_none() {
                    continue;
                }

                let text: String = element.text().collect();
                if !text.contains("홈페이지") {
                    continue;
                }

                if let Some(href) = element.value().attr("href") {
                    if href.starts_with("http") && !href.starts_with('#') {
                        if let Some(dept_name) = pending_dept.take() {
                            results.push((current_college.clone(), dept_name, href.to_string()));
                        }
                    }
                }
            }
        }

        // Handle the last pending department
        if let Some(dept_name) = pending_dept.take() {
            results.push((current_college.clone(), dept_name, "NOT_FOUND".to_string()));
        }

        results
    }

    fn clean_header_text(&self, header: ElementRef) -> String {
        let mut text: String = header.text().collect();
        for suffix in &["교수진", "홈페이지"] {
            if let Some(idx) = text.find(suffix) {
                text = text[..idx].to_string();
            }
        }
        text.trim().to_string()
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
