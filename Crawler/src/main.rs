// src/main.rs

mod config;
mod locale;
mod models;
mod utils;

use clap::Parser;
use reqwest::Client;
use scraper::{Html, Selector};

use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::models::config::{Config, LocaleConfig};
use crate::models::crawler::{BoardConfig, Campus, Department, Notice};

use crate::config::{clean_date, clean_title, format_notice};
use crate::locale::load_locale_or_default;
use crate::utils::resolve_url;

/// Fetch notices from university department websites
#[derive(Parser, Debug)]
#[command(name = "crawler")]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "data/config.toml")]
    config: String,

    /// Path to locale file
    #[arg(long, default_value = "data/locale.toml")]
    locale: String,

    /// Override site map path
    #[arg(long)]
    site_map: Option<String>,

    /// Override output path
    #[arg(short, long)]
    output: Option<String>,

    /// Suppress console output
    #[arg(short, long)]
    quiet: bool,
}

/// Load campus configurations from a JSON file
fn load_campuses<P: AsRef<Path>>(path: P) -> Result<Vec<Campus>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let campuses: Vec<Campus> = serde_json::from_str(&content)?;
    Ok(campuses)
}

/// Fetch notices from a single board
async fn fetch_board_notices(
    campus: &str,
    department: &Department,
    board: &BoardConfig,
    client: &Client,
    config: &Config,
) -> Result<Vec<Notice>, Box<dyn Error>> {
    let html_content = client.get(&board.url).send().await?.text().await?;
    let document = Html::parse_document(&html_content);

    // Parse selectors (compile once for performance)
    let row_sel = Selector::parse(&board.row_selector).unwrap();
    let title_sel = Selector::parse(&board.title_selector).unwrap();
    let date_sel = Selector::parse(&board.date_selector).unwrap();

    // Extract base URL for resolving relative links
    let base_url = url::Url::parse(&board.url)?;

    let mut notices = Vec::new();

    for row in document.select(&row_sel) {
        let title_elem = row.select(&title_sel).next();
        let date_elem = row.select(&date_sel).next();

        if let (Some(t), Some(d)) = (title_elem, date_elem) {
            // Normalize whitespace in title and date using config patterns
            let title = clean_title(&t.text().collect::<Vec<_>>().join(" "), &config.cleaning);
            let date = clean_date(&d.text().collect::<Vec<_>>().join(" "), &config.cleaning);

            // Resolve relative URLs to absolute
            // Use link_selector if provided, otherwise use title_selector
            let link_elem = if let Some(ref link_sel_str) = board.link_selector {
                let link_sel = Selector::parse(link_sel_str).unwrap();
                row.select(&link_sel).next()
            } else {
                Some(t)
            };

            let raw_link = link_elem
                .and_then(|e| e.value().attr(&board.attr_name))
                .unwrap_or("");
            let link = resolve_url(&base_url, raw_link);

            if !title.is_empty() {
                notices.push(Notice {
                    campus: campus.to_string(),
                    department_id: department.id.clone(),
                    department_name: department.name.clone(),
                    board_id: board.id.clone(),
                    board_name: board.name.clone(),
                    title,
                    date,
                    link,
                });
            }
        }
    }
    Ok(notices)
}

/// Fetch notices from all departments and their boards
async fn fetch_all_notices(
    campuses: &[Campus],
    config: &Config,
    locale: &LocaleConfig,
) -> Vec<Notice> {
    let client = Client::builder()
        .user_agent(&config.crawler.user_agent)
        .timeout(Duration::from_secs(config.crawler.timeout_secs))
        .build()
        .expect("Failed to build HTTP client");

    let mut all_notices = Vec::new();
    let delay = Duration::from_millis(config.crawler.request_delay_ms);

    for campus in campuses {
        for dept in &campus.departments {
            if config.logging.show_progress {
                println!(
                    "{}",
                    locale
                        .messages
                        .department_header
                        .replace("{dept_name}", &dept.name)
                );
            }
            for board in &dept.boards {
                match fetch_board_notices(&campus.campus, dept, board, &client, config).await {
                    Ok(notices) => {
                        if config.logging.show_progress {
                            println!(
                                "{}",
                                locale
                                    .messages
                                    .board_success
                                    .replace("{board_name}", &board.name)
                                    .replace("{notice_count}", &notices.len().to_string())
                            );
                        }
                        all_notices.extend(notices);
                    }
                    Err(e) => {
                        eprintln!(
                            "{}",
                            locale
                                .messages
                                .board_error
                                .replace("{board_name}", &board.name)
                                .replace("{error_msg}", &format!("{}", e))
                        );
                    }
                }
                // Polite delay between requests
                if config.crawler.request_delay_ms > 0 {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    all_notices
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Load locale configuration first (using default path initially)
    let mut locale = load_locale_or_default("data/locale.toml");

    // Load configuration
    let mut config = Config::load_or_default(&args.config, &locale);

    // If custom locale path specified, reload locale
    if args.locale != "data/locale.toml" {
        locale = load_locale_or_default(&args.locale);
    }

    // Apply CLI overrides
    if let Some(site_map) = args.site_map {
        config.paths.site_map = site_map;
    }
    if let Some(output) = args.output {
        config.paths.output = output;
    }
    if args.quiet {
        config.output.console_enabled = false;
        config.logging.show_progress = false;
    }

    if config.logging.show_progress {
        print!("{}", locale.messages.crawler_starting);
    }

    // Load campus configurations
    let campuses = load_campuses(&config.paths.site_map)?;

    let total_boards: usize = campuses
        .iter()
        .map(|c| c.departments.iter().map(|d| d.boards.len()).sum::<usize>())
        .sum();
    if config.logging.show_progress {
        println!(
            "{}",
            locale
                .messages
                .loaded_departments
                .replace("{count_dept}", &campuses.len().to_string())
                .replace("{count_board}", &total_boards.to_string())
        );
    }

    // Fetch notices from all campuses
    let notices = fetch_all_notices(&campuses, &config, &locale).await;

    // Display results to console
    if config.output.console_enabled {
        println!(
            "{}",
            locale
                .messages
                .total_notices
                .replace("{total_count}", &notices.len().to_string())
        );
        println!("{:=<80}", locale.messages.separator_line);

        for notice in &notices {
            let formatted = format_notice(
                &config.output.notice_format,
                &notice.department_name,
                &notice.board_name,
                &notice.title,
                &notice.date,
                &notice.link,
            );
            println!("{}", formatted);
            println!("{:-<80}", locale.messages.separator_short);
        }
    }

    // Save notices to JSON files (organized by campusName/department/board)
    if config.output.json_enabled {
        // Create output directory if it doesn't exist
        std::fs::create_dir_all(&config.paths.output)?;

        // Group notices by campus, department, and board
        use std::collections::HashMap;
        let mut notices_by_campus: HashMap<String, HashMap<String, HashMap<String, Vec<&Notice>>>> =
            HashMap::new();

        for notice in &notices {
            notices_by_campus
                .entry(notice.campus.clone())
                .or_insert_with(HashMap::new)
                .entry(notice.department_name.clone())
                .or_insert_with(HashMap::new)
                .entry(notice.board_name.clone())
                .or_insert_with(Vec::new)
                .push(notice);
        }

        // Write JSON files for each campus/department/board
        for (campus_name, departments) in notices_by_campus {
            let campus_dir = Path::new(&config.paths.output).join(&campus_name);
            std::fs::create_dir_all(&campus_dir)?;

            for (dept_name, boards) in departments {
                let dept_dir = campus_dir.join(&dept_name);
                std::fs::create_dir_all(&dept_dir)?;

                for (board_name, board_notices) in boards {
                    // Sanitize board name for filename (replace special characters)
                    let safe_board_name = board_name
                        .replace("/", "-")
                        .replace("\\", "-")
                        .replace(":", "-")
                        .replace("*", "-")
                        .replace("?", "-")
                        .replace("\"", "-")
                        .replace("<", "-")
                        .replace(">", "-")
                        .replace("|", "-");

                    let file_path = dept_dir.join(format!("{}.json", safe_board_name));

                    let json_output = if config.output.json_pretty {
                        serde_json::to_string_pretty(&board_notices)?
                    } else {
                        serde_json::to_string(&board_notices)?
                    };

                    fs::write(&file_path, &json_output)?;
                }
            }
        }

        if config.logging.show_progress {
            println!(
                "{}",
                locale
                    .messages
                    .saved_notices
                    .replace("{output_path}", &config.paths.output)
            );
        }
    }

    Ok(())
}
