//! Notice crawler service.
//!
//! Fetches notices from department boards using configured CSS selectors.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, StreamExt};
use reqwest::Client;
use scraper::Selector;

use crate::error::{AppError, Result};
use crate::models::{
    Board, Campus, Config, CrawlError, CrawlOutcome, CrawlStage, DepartmentRef, Notice,
};
use crate::utils::{extract_notice_id, http, resolve_url};

/// Board selectors for notice extraction.
#[derive(Clone)]
struct BoardSelectors {
    row: Selector,
    title: Selector,
    date: Selector,
    author: Option<Selector>,
    link: Option<Selector>,
}

/// Result of fetching a board's notice list.
struct BoardListResult {
    notices: Vec<Notice>,
    row_total: usize,
    row_failures: usize,
}

/// Service for crawling notices from department boards.
pub struct NoticeCrawler {
    config: Arc<Config>,
    client: Client,
}

/// Implementation of NoticeCrawler
impl NoticeCrawler {
    /// Create a new notice crawler with the given configuration.
    pub fn new(config: Arc<Config>, client: Client) -> Result<Self> {
        Ok(Self { config, client })
    }

    /// Fetch all notices from all campuses concurrently.
    pub async fn fetch_all(&self, campuses: &[Campus]) -> Result<CrawlOutcome> {
        let concurrency = self.config.crawler.max_concurrent.max(1);
        let board_lookup = Arc::new(Self::build_board_lookup(campuses));
        let (selector_cache, selector_errors, invalid_boards) =
            Self::build_selector_cache(campuses);
        let selector_cache = Arc::new(selector_cache);

        // Stage 1: Fetch all notice lists from boards concurrently, but bounded by concurrency.
        let board_jobs_all: Vec<_> = campuses
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
        let board_jobs: Vec<_> = board_jobs_all
            .into_iter()
            .filter(|(_, board)| !invalid_boards.contains(board.id.as_str()))
            .collect();

        let mut outcome = CrawlOutcome {
            board_total: board_jobs.len() + invalid_boards.len(),
            board_failures: invalid_boards.len(),
            errors: selector_errors,
            ..CrawlOutcome::default()
        };

        let mut notice_buffer = Vec::new();
        let mut board_stream = stream::iter(board_jobs)
            .map(|(dept_ref, board)| {
                let selector_cache = Arc::clone(&selector_cache);
                async move {
                    let selectors = selector_cache.get(&board.id).cloned().ok_or_else(|| {
                        AppError::crawl("selector_cache", "Missing selector cache entry")
                    });
                    let result = match selectors {
                        Ok(selectors) => self.fetch_board_list(dept_ref, board, &selectors).await,
                        Err(err) => Err(err),
                    };
                    (board, result)
                }
            })
            .buffer_unordered(concurrency);

        while let Some((board, result)) = board_stream.next().await {
            match result {
                Ok(list_result) => {
                    outcome.notice_total += list_result.row_total;
                    outcome.notice_failures += list_result.row_failures;
                    notice_buffer.extend(list_result.notices);
                }
                Err(error) => {
                    outcome.board_failures += 1;
                    outcome.errors.push(Self::build_error(
                        CrawlStage::BoardList,
                        Some(board),
                        Some(&board.url),
                        None,
                        &error,
                    ));
                    log::warn!(
                        "Failed to fetch board list {} ({}): {}",
                        board.name,
                        board.url,
                        error
                    );
                }
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
                let selector_cache = Arc::clone(&selector_cache);
                let notice_id = notice.canonical_id();
                let board_id = notice.board_id.clone();
                let board_name = notice.board_name.clone();
                let url = notice.link.clone();
                async move {
                    let result = self
                        .fetch_notice_detail(notice, &board_lookup, &selector_cache)
                        .await;
                    (notice_id, board_id, board_name, url, result)
                }
            })
            .buffer_unordered(concurrency);

        let mut detailed = Vec::new();
        let mut detail_stream = detailed_notices;
        while let Some((notice_id, board_id, board_name, url, result)) = detail_stream.next().await
        {
            match result {
                Ok(notice) => detailed.push(notice),
                Err(error) => {
                    outcome.detail_failures += 1;
                    let stage = if matches!(
                        &error,
                        AppError::Crawl { context, .. } if context == "find_board"
                    ) {
                        CrawlStage::BoardLookup
                    } else {
                        CrawlStage::NoticeDetail
                    };
                    outcome.errors.push(CrawlError {
                        stage,
                        board_id: Some(board_id),
                        board_name: Some(board_name),
                        url: Some(url),
                        notice_id: Some(notice_id),
                        message: error.to_string(),
                        retryable: error.is_retryable(),
                    });
                    log::warn!("Failed to fetch notice detail: {}", error);
                }
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
        selectors: &BoardSelectors,
    ) -> Result<BoardListResult> {
        self.apply_request_delay().await;
        let document = http::fetch_page_async(&self.client, &board.url).await?;
        let base_url = url::Url::parse(&board.url)?;
        let mut notices = Vec::new();
        let mut row_total = 0;
        let mut row_failures = 0;

        for row in document.select(&selectors.row) {
            row_total += 1;
            if let Some(notice) = self.parse_notice_row(
                &row,
                selectors,
                &board.selectors.attr_name,
                dept_ref,
                board,
                &base_url,
            ) {
                notices.push(notice);
            } else {
                row_failures += 1;
            }
        }
        Ok(BoardListResult {
            notices,
            row_total,
            row_failures,
        })
    }

    /// Process a single notice (placeholder for future detail fetching).
    async fn fetch_notice_detail(
        &self,
        notice: Notice,
        _board_lookup: &HashMap<&str, &Board>,
        _selector_cache: &HashMap<String, Arc<BoardSelectors>>,
    ) -> Result<Notice> {
        // Note: Body content is no longer stored in the notice.
        // This method is kept for future pinned detection or other metadata
        Ok(notice)
    }

    #[allow(clippy::too_many_arguments)]
    fn parse_notice_row(
        &self,
        row: &scraper::ElementRef,
        selectors: &BoardSelectors,
        attr_name: &str,
        dept_ref: DepartmentRef<'_>,
        board: &Board,
        base_url: &url::Url,
    ) -> Option<Notice> {
        let title_elem = row.select(&selectors.title).next()?;
        let date_elem = row.select(&selectors.date).next()?;
        let author_elem = selectors
            .author
            .as_ref()
            .and_then(|sel| row.select(sel).next());

        let raw_title: String = title_elem.text().collect();
        let raw_date: String = date_elem.text().collect();
        let raw_author: String = author_elem.map_or(String::new(), |el| el.text().collect());

        let title = self.config.cleaning.clean_title(&raw_title);
        let date = self.config.cleaning.clean_date(&raw_date);

        if title.is_empty() {
            return None;
        }

        let link_elem = selectors
            .link
            .as_ref()
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
            is_pinned: false, // TODO: Detect pinned notices from row styling
        })
    }

    async fn apply_request_delay(&self) {
        let delay_ms = self.config.crawler.request_delay_ms;
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    fn build_selector_cache(
        campuses: &[Campus],
    ) -> (
        HashMap<String, Arc<BoardSelectors>>,
        Vec<CrawlError>,
        HashSet<String>,
    ) {
        let mut cache = HashMap::new();
        let mut errors = Vec::new();
        let mut invalid_boards = HashSet::new();

        for campus in campuses {
            for dept_ref in campus.all_departments() {
                for board in &dept_ref.dept.boards {
                    let row = match Self::parse_selector(&board.selectors.row_selector) {
                        Ok(sel) => sel,
                        Err(err) => {
                            errors.push(Self::build_error(
                                CrawlStage::Selector,
                                Some(board),
                                Some(&board.url),
                                None,
                                &err,
                            ));
                            invalid_boards.insert(board.id.clone());
                            continue;
                        }
                    };
                    let title = match Self::parse_selector(&board.selectors.title_selector) {
                        Ok(sel) => sel,
                        Err(err) => {
                            errors.push(Self::build_error(
                                CrawlStage::Selector,
                                Some(board),
                                Some(&board.url),
                                None,
                                &err,
                            ));
                            invalid_boards.insert(board.id.clone());
                            continue;
                        }
                    };
                    let date = match Self::parse_selector(&board.selectors.date_selector) {
                        Ok(sel) => sel,
                        Err(err) => {
                            errors.push(Self::build_error(
                                CrawlStage::Selector,
                                Some(board),
                                Some(&board.url),
                                None,
                                &err,
                            ));
                            invalid_boards.insert(board.id.clone());
                            continue;
                        }
                    };
                    let author = match board.selectors.author_selector.as_ref() {
                        Some(sel) => match Self::parse_selector(sel) {
                            Ok(parsed) => Some(parsed),
                            Err(err) => {
                                errors.push(Self::build_error(
                                    CrawlStage::Selector,
                                    Some(board),
                                    Some(&board.url),
                                    None,
                                    &err,
                                ));
                                None
                            }
                        },
                        None => None,
                    };
                    let link = match board.selectors.link_selector.as_ref() {
                        Some(sel) => match Self::parse_selector(sel) {
                            Ok(parsed) => Some(parsed),
                            Err(err) => {
                                errors.push(Self::build_error(
                                    CrawlStage::Selector,
                                    Some(board),
                                    Some(&board.url),
                                    None,
                                    &err,
                                ));
                                None
                            }
                        },
                        None => None,
                    };

                    cache.insert(
                        board.id.clone(),
                        Arc::new(BoardSelectors {
                            row,
                            title,
                            date,
                            author,
                            link,
                        }),
                    );
                }
            }
        }

        (cache, errors, invalid_boards)
    }

    fn build_error(
        stage: CrawlStage,
        board: Option<&Board>,
        url: Option<&str>,
        notice_id: Option<&str>,
        error: &AppError,
    ) -> CrawlError {
        CrawlError {
            stage,
            board_id: board.map(|b| b.id.clone()),
            board_name: board.map(|b| b.name.clone()),
            url: url.map(str::to_string),
            notice_id: notice_id.map(str::to_string),
            message: error.to_string(),
            retryable: error.is_retryable(),
        }
    }

    fn build_board_lookup<'a>(campuses: &'a [Campus]) -> HashMap<&'a str, &'a Board> {
        campuses
            .iter()
            .flat_map(|campus| campus.all_departments())
            .flat_map(|dept_ref| dept_ref.dept.boards.iter())
            .map(|board| (board.id.as_str(), board))
            .collect()
    }

    #[allow(dead_code)]
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
