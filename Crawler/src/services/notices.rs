// src/services/notices.rs

//! Notice crawler service.
//!
//! Fetches notices from department boards using configured CSS selectors.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, StreamExt};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::error::{AppError, Result};
use crate::models::{Board, Campus, Config, DepartmentRef, Notice};
use crate::utils::log;
use crate::utils::resolve_url;
use crate::utils::url::extract_notice_id;

/// Summary of a crawl run.
#[derive(Debug, Default)]
pub struct CrawlOutcome {
    pub notices: Vec<Notice>,
    pub board_total: usize,
    pub board_failures: usize,
    pub detail_total: usize,
    pub detail_failures: usize,
}

/// Service for crawling notices from department boards.
pub struct NoticeCrawler {
    config: Arc<Config>,
    client: Client,
}

impl NoticeCrawler {
    /// Create a new notice crawler with the given configuration.
    pub fn new(config: Arc<Config>) -> Self {
        let client = Client::builder()
            .user_agent(&config.crawler.user_agent)
            .timeout(Duration::from_secs(config.crawler.timeout_secs))
            .build()
            .expect("Failed to build HTTP client");

        Self { config, client }
    }

    /// Fetch all notices from all campuses concurrently.
    pub async fn fetch_all(&self, campuses: &[Campus]) -> Result<CrawlOutcome> {
        let delay = Duration::from_millis(self.config.crawler.request_delay_ms);
        let concurrency = self.config.crawler.max_concurrent.max(1);
        let board_lookup = Arc::new(Self::build_board_lookup(campuses));

        // Stage 1: Fetch all notice lists from boards concurrently, but bounded by concurrency.
        let board_jobs: Vec<_> = campuses
            .iter()
            .flat_map(|c| c.all_departments())
            .flat_map(|dept_ref| {
                dept_ref
                    .dept
                    .boards
                    .iter()
                    .map(move |board| (dept_ref, board))
            })
            .collect();

        let mut outcome = CrawlOutcome {
            board_total: board_jobs.len(),
            ..CrawlOutcome::default()
        };

        let mut notice_buffer = Vec::new();
        let mut board_stream = stream::iter(board_jobs)
            .map(|(dept_ref, board)| async move {
                let result = self.fetch_board_list(dept_ref, board).await;
                (board, result)
            })
            .buffer_unordered(concurrency);

        while let Some((board, result)) = board_stream.next().await {
            match result {
                Ok(notices) => notice_buffer.extend(notices),
                Err(error) => {
                    outcome.board_failures += 1;
                    log::warn(&format!(
                        "Failed to fetch board list {} ({}): {}",
                        board.name, board.url, error
                    ));
                }
            }

            if delay.as_millis() > 0 {
                tokio::time::sleep(delay).await;
            }
        }

        let mut seen = HashSet::new();
        let mut deduped = Vec::new();
        for notice in notice_buffer {
            let id = notice.canonical_id();
            if seen.insert(id) {
                deduped.push(notice);
            }
        }

        // Stage 2: Fetch details for each notice concurrently.
        outcome.detail_total = deduped.len();
        let detailed_notices = stream::iter(deduped)
            .map(|notice| {
                let board_lookup = Arc::clone(&board_lookup);
                async move { self.fetch_notice_detail(notice, &board_lookup).await }
            })
            .buffered(concurrency);

        let mut detailed = Vec::new();
        let mut detail_stream = detailed_notices;
        while let Some(result) = detail_stream.next().await {
            match result {
                Ok(notice) => detailed.push(notice),
                Err(error) => {
                    outcome.detail_failures += 1;
                    log::warn(&format!("Failed to fetch notice detail: {}", error));
                }
            }

            if delay.as_millis() > 0 {
                tokio::time::sleep(delay).await;
            }
        }

        outcome.notices = detailed;
        Ok(outcome)
    }

    /// Fetch a list of notices from a single board.
    async fn fetch_board_list(
        &self,
        dept_ref: DepartmentRef<'_>,
        board: &Board,
    ) -> Result<Vec<Notice>> {
        let html = self.client.get(&board.url).send().await?.text().await?;
        let document = Html::parse_document(&html);

        let row_sel = Self::parse_selector(&board.selectors.row_selector)?;
        let title_sel = Self::parse_selector(&board.selectors.title_selector)?;
        let date_sel = Self::parse_selector(&board.selectors.date_selector)?;
        let author_sel = board
            .selectors
            .author_selector
            .as_ref()
            .map(|s| Self::parse_selector(s))
            .transpose()?;
        let link_sel = board
            .selectors
            .link_selector
            .as_ref()
            .map(|s| Self::parse_selector(s))
            .transpose()?;

        let base_url = url::Url::parse(&board.url)?;
        let mut notices = Vec::new();

        for row in document.select(&row_sel) {
            if let Some(notice) = self.parse_notice_row(
                &row,
                &title_sel,
                &date_sel,
                author_sel.as_ref(),
                link_sel.as_ref(),
                &board.selectors.attr_name,
                dept_ref,
                board,
                &base_url,
            ) {
                notices.push(notice);
            }
        }
        Ok(notices)
    }

    /// Fetch the body for a single notice.
    async fn fetch_notice_detail(
        &self,
        mut notice: Notice,
        board_lookup: &HashMap<&str, &Board>,
    ) -> Result<Notice> {
        let board = self.find_board(&notice, board_lookup)?;
        if let Some(body_selector_str) = &board.selectors.body_selector {
            if !notice.link.is_empty() {
                let html = self.client.get(&notice.link).send().await?.text().await?;
                let document = Html::parse_document(&html);
                let body_sel = Self::parse_selector(body_selector_str)?;
                if let Some(body_elem) = document.select(&body_sel).next() {
                    notice.body = body_elem.inner_html();
                }
            }
        }
        Ok(notice)
    }

    #[allow(clippy::too_many_arguments)]
    fn parse_notice_row(
        &self,
        row: &scraper::ElementRef,
        title_sel: &Selector,
        date_sel: &Selector,
        author_sel: Option<&Selector>,
        link_sel: Option<&Selector>,
        attr_name: &str,
        dept_ref: DepartmentRef<'_>,
        board: &Board,
        base_url: &url::Url,
    ) -> Option<Notice> {
        let title_elem = row.select(title_sel).next()?;
        let date_elem = row.select(date_sel).next()?;
        let author_elem = author_sel.and_then(|sel| row.select(sel).next());

        let raw_title: String = title_elem.text().collect();
        let raw_date: String = date_elem.text().collect();
        let raw_author: String = author_elem.map_or(String::new(), |el| el.text().collect());

        let title = self.config.cleaning.clean_title(&raw_title);
        let date = self.config.cleaning.clean_date(&raw_date);

        if title.is_empty() {
            return None;
        }

        let link_elem = link_sel
            .and_then(|sel| row.select(sel).next())
            .or(Some(title_elem));
        let raw_link = link_elem
            .and_then(|e| e.value().attr(attr_name))
            .unwrap_or("");
        let link = resolve_url(base_url, raw_link);
        let source_id = extract_notice_id(&link);

        Some(Notice {
            campus: dept_ref.campus.to_string(),
            college: dept_ref.college.unwrap_or("").to_string(),
            department_id: dept_ref.dept.id.clone(),
            department_name: dept_ref.dept.name.clone(),
            board_id: board.id.clone(),
            board_name: board.name.clone(),
            title,
            author: raw_author.trim().to_string(),
            date,
            link,
            source_id,
            body: String::new(), // Body will be fetched later
        })
    }

    fn build_board_lookup<'a>(campuses: &'a [Campus]) -> HashMap<&'a str, &'a Board> {
        campuses
            .iter()
            .flat_map(|campus| campus.all_departments())
            .flat_map(|dept_ref| dept_ref.dept.boards.iter())
            .map(|board| (board.id.as_str(), board))
            .collect()
    }

    fn find_board<'a>(
        &self,
        notice: &Notice,
        board_lookup: &'a HashMap<&str, &'a Board>,
    ) -> Result<&'a Board> {
        board_lookup
            .get(notice.board_id.as_str())
            .copied()
            .ok_or_else(|| AppError::Crawl {
                context: "find_board".to_string(),
                message: format!("Board with id {} not found", notice.board_id),
            })
    }

    fn parse_selector(s: &str) -> Result<Selector> {
        Selector::parse(s).map_err(|e| AppError::selector(s, format!("{e:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_selector_valid() {
        assert!(NoticeCrawler::parse_selector("div.class").is_ok());
        assert!(NoticeCrawler::parse_selector("tr:has(a)").is_ok());
    }

    #[test]
    fn test_parse_selector_invalid() {
        assert!(NoticeCrawler::parse_selector("[[invalid").is_err());
    }
}
