// src/services/notices.rs

//! Notice crawler service.
//!
//! Fetches notices from department boards using configured CSS selectors.

use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, StreamExt};
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::Mutex;

use crate::error::{AppError, Result};
use crate::models::{Board, Campus, Config, DepartmentRef, Notice};
use crate::utils::{log, resolve_url};

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
    pub async fn fetch_all(&self, campuses: &[Campus]) -> Result<Vec<Notice>> {
        let all_notices = Arc::new(Mutex::new(Vec::new()));
        let delay = Duration::from_millis(self.config.crawler.request_delay_ms);

        // Flatten all boards with their context
        let tasks: Vec<_> = campuses
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

        let concurrency = self.config.crawler.max_concurrent.max(1);

        stream::iter(tasks)
            .for_each_concurrent(concurrency, |(dept_ref, board)| {
                let notices = Arc::clone(&all_notices);
                let client = self.client.clone();
                let config = Arc::clone(&self.config);

                async move {
                    match Self::fetch_board(&client, &config, dept_ref, board).await {
                        Ok(board_notices) => {
                            notices.lock().await.extend(board_notices);
                        }
                        Err(e) => {
                            log::error(&format!(
                                "Error fetching {}/{}: {e}",
                                dept_ref.dept.name, board.name
                            ));
                        }
                    }

                    if delay.as_millis() > 0 {
                        tokio::time::sleep(delay).await;
                    }
                }
            })
            .await;

        Ok(Arc::try_unwrap(all_notices)
            .expect("Arc still has multiple owners")
            .into_inner())
    }

    /// Fetch notices from a single board.
    async fn fetch_board(
        client: &Client,
        config: &Config,
        dept_ref: DepartmentRef<'_>,
        board: &Board,
    ) -> Result<Vec<Notice>> {
        let html = client.get(&board.url).send().await?.text().await?;
        let document = Html::parse_document(&html);

        let row_sel = Self::parse_selector(&board.selectors.row_selector)?;
        let title_sel = Self::parse_selector(&board.selectors.title_selector)?;
        let date_sel = Self::parse_selector(&board.selectors.date_selector)?;
        let link_sel = board
            .selectors
            .link_selector
            .as_ref()
            .map(|s| Self::parse_selector(s))
            .transpose()?;

        let base_url = url::Url::parse(&board.url)?;
        let mut notices = Vec::new();

        for row in document.select(&row_sel) {
            if let Some(notice) = Self::parse_notice_row(
                &row,
                &title_sel,
                &date_sel,
                link_sel.as_ref(),
                &board.selectors.attr_name,
                config,
                dept_ref,
                board,
                &base_url,
            ) {
                notices.push(notice);
            }
        }

        Ok(notices)
    }

    #[allow(clippy::too_many_arguments)]
    fn parse_notice_row(
        row: &scraper::ElementRef,
        title_sel: &Selector,
        date_sel: &Selector,
        link_sel: Option<&Selector>,
        attr_name: &str,
        config: &Config,
        dept_ref: DepartmentRef<'_>,
        board: &Board,
        base_url: &url::Url,
    ) -> Option<Notice> {
        let title_elem = row.select(title_sel).next()?;
        let date_elem = row.select(date_sel).next()?;

        let raw_title: String = title_elem.text().collect();
        let raw_date: String = date_elem.text().collect();

        let title = config.cleaning.clean_title(&raw_title);
        let date = config.cleaning.clean_date(&raw_date);

        if title.is_empty() {
            return None;
        }

        // Get link element (from link_selector or fallback to title element)
        let link_elem = link_sel
            .and_then(|sel| row.select(sel).next())
            .or(Some(title_elem));

        let raw_link = link_elem
            .and_then(|e| e.value().attr(attr_name))
            .unwrap_or("");

        Some(Notice {
            campus: dept_ref.campus.to_string(),
            college: dept_ref.college.unwrap_or("").to_string(),
            department_id: dept_ref.dept.id.clone(),
            department_name: dept_ref.dept.name.clone(),
            board_id: board.id.clone(),
            board_name: board.name.clone(),
            title,
            date,
            link: resolve_url(base_url, raw_link),
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
