//! Department crawler service.

use regex::Regex;
use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Selector};

use crate::error::Result;
use crate::models::{Campus, CampusInfo, College, Department};
use crate::utils::http::fetch_page;

/// Service for crawling campus department information
pub struct DepartmentCrawler<'a> {
    client: &'a Client,
}

impl<'a> DepartmentCrawler<'a> {
    /// Create a new department crawler
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Crawl all campuses
    pub fn crawl_all(&self, campuses: &[CampusInfo]) -> Result<Vec<Campus>> {
        let mut results = Vec::new();

        for campus_info in campuses {
            println!("Crawling {}...", campus_info.name);
            match self.crawl_campus(campus_info) {
                Ok(campus) => {
                    let dept_count: usize =
                        campus.colleges.iter().map(|c| c.departments.len()).sum();
                    println!("  Found {} departments", dept_count);
                    results.push(campus);
                }
                Err(e) => {
                    println!("  Failed to crawl {}: {}", campus_info.name, e);
                }
            }
        }

        Ok(results)
    }

    /// Crawl a single campus
    fn crawl_campus(&self, info: &CampusInfo) -> Result<Campus> {
        let document = fetch_page(self.client, &info.url)?;

        let mut campus = Campus {
            campus: info.name.clone(),
            colleges: Vec::new(),
        };

        let main_selector = Selector::parse("main").unwrap();

        let main = match document.select(&main_selector).next() {
            Some(m) => m,
            None => {
                println!("  Cannot find main content area for {}", info.name);
                return Ok(campus);
            }
        };

        // Extract all departments with their colleges and URLs
        let dept_info = self.extract_departments_from_main(main, &document);

        // Group by college
        for (college_name, dept_name, dept_url) in dept_info {
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

            let dept_id = Self::generate_department_id(&dept_name, &dept_url);
            campus.colleges[college_idx].departments.push(Department {
                id: dept_id,
                name: dept_name,
                url: dept_url,
                boards: Vec::new(),
            });
        }

        Ok(campus)
    }

    /// Extract departments from main element
    fn extract_departments_from_main(
        &self,
        main_elem: ElementRef,
        document: &Html,
    ) -> Vec<(String, String, String)> {
        let h1_selector = Selector::parse("h1").unwrap();
        let college_pattern = Regex::new(r"([가-힣]+대학)$").unwrap();

        let mut results: Vec<(String, String, String)> = Vec::new();
        let mut current_college = String::new();

        // Get all homepage URLs
        let homepage_urls = self.extract_all_homepage_urls(document);
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

    /// Find all homepage URLs in the document
    fn extract_all_homepage_urls(&self, document: &Html) -> Vec<(usize, String)> {
        let link_selector = Selector::parse("a").unwrap();
        let mut urls = Vec::new();
        let html = document.html();

        for element in document.select(&link_selector) {
            let text: String = element.text().collect();
            if text.contains("홈페이지") {
                if let Some(href) = element.value().attr("href") {
                    if href.starts_with("http") && !href.starts_with('#') {
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
}
