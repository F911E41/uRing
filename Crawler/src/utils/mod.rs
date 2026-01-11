// src/utils/mod.rs

//! Utility functions and helpers.

pub mod fs;
pub mod http;
pub mod log;
pub mod url;

use std::collections::HashMap;
use std::path::Path;

use crate::error::Result;
use crate::models::{Config, LocaleConfig, Notice};

/// Resolve a potentially relative URL against a base URL.
pub fn resolve_url(base: &::url::Url, href: &str) -> String {
    base.join(href)
        .map(|u: ::url::Url| u.to_string())
        .unwrap_or_else(|_| href.to_string())
}

/// Save notices to JSON files, organized by campus/department/board.
pub fn save_notices(notices: &[Notice], config: &Config, locale: &LocaleConfig) -> Result<()> {
    if !config.output.json_enabled {
        return Ok(());
    }

    let output_path = Path::new(&config.paths.output);
    fs::create_dir_all(output_path)?;

    // Group notices hierarchically
    let grouped = group_notices(notices);

    // Save each group to a file
    for (campus, departments) in grouped {
        let campus_dir = output_path.join(&campus);
        fs::create_dir_all(&campus_dir)?;

        for (dept, boards) in departments {
            let dept_dir = campus_dir.join(&dept);
            fs::create_dir_all(&dept_dir)?;

            for (board, board_notices) in boards {
                save_board_notices(&dept_dir, &board, &board_notices, config)?;
            }
        }
    }

    if config.logging.show_progress {
        log::info(&format!(
            "{}",
            locale
                .messages
                .saved_notices
                .replace("{output_path}", &config.paths.output)
        ));
    }

    Ok(())
}

type NoticeGroups<'a> = HashMap<&'a str, HashMap<&'a str, HashMap<&'a str, Vec<&'a Notice>>>>;

fn group_notices(notices: &[Notice]) -> NoticeGroups<'_> {
    let mut grouped: NoticeGroups = HashMap::new();

    for notice in notices {
        grouped
            .entry(&notice.campus)
            .or_default()
            .entry(&notice.department_name)
            .or_default()
            .entry(&notice.board_name)
            .or_default()
            .push(notice);
    }

    grouped
}

fn save_board_notices(
    dept_dir: &Path,
    board: &str,
    notices: &[&Notice],
    config: &Config,
) -> Result<()> {
    let safe_name = board.replace(|c: char| !c.is_alphanumeric(), "-");
    let file_path = dept_dir.join(format!("{safe_name}.json"));

    let json = if config.output.json_pretty {
        serde_json::to_string_pretty(notices)?
    } else {
        serde_json::to_string(notices)?
    };

    fs::write(&file_path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_url() {
        let base = ::url::Url::parse("https://example.com/path/").unwrap();
        assert_eq!(
            resolve_url(&base, "page.html"),
            "https://example.com/path/page.html"
        );
        assert_eq!(
            resolve_url(&base, "/root.html"),
            "https://example.com/root.html"
        );
        assert_eq!(
            resolve_url(&base, "https://other.com/x"),
            "https://other.com/x"
        );
    }
}
