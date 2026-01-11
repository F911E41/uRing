// src/logging.rs

//! Centralized logging module with server-style formatting.
//!
//! Provides consistent log output with timestamps and log levels.

#![allow(dead_code)]

use std::sync::OnceLock;

use chrono::Local;

use crate::models::LocaleConfig;

/// Global locale configuration for logging
static LOCALE: OnceLock<LocaleConfig> = OnceLock::new();

/// Log level enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

/// Current log level
static LOG_LEVEL: OnceLock<LogLevel> = OnceLock::new();

/// Initialize the logging system with locale and level
pub fn init(locale: &LocaleConfig, level: &str) {
    let _ = LOCALE.set(locale.clone());
    let _ = LOG_LEVEL.set(LogLevel::from_str(level));
}

/// Get the current locale config
pub fn locale() -> &'static LocaleConfig {
    LOCALE.get_or_init(LocaleConfig::default)
}

/// Check if a log level should be displayed
fn should_log(level: LogLevel) -> bool {
    let current = LOG_LEVEL.get().copied().unwrap_or(LogLevel::Info);
    level >= current
}

/// Format a log message with timestamp and level
fn format_log(level: LogLevel, message: &str) -> String {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    format!("[{}] [{}] {}", timestamp, level.as_str(), message)
}

/// Log a debug message
pub fn debug(message: &str) {
    if should_log(LogLevel::Debug) {
        eprintln!("{}", format_log(LogLevel::Debug, message));
    }
}

/// Log an info message
pub fn info(message: &str) {
    if should_log(LogLevel::Info) {
        println!("{}", format_log(LogLevel::Info, message));
    }
}

/// Log a warning message
pub fn warn(message: &str) {
    if should_log(LogLevel::Warn) {
        eprintln!("{}", format_log(LogLevel::Warn, message));
    }
}

/// Log an error message
pub fn error(message: &str) {
    if should_log(LogLevel::Error) {
        eprintln!("{}", format_log(LogLevel::Error, message));
    }
}

/// Log a success message (always shown as INFO)
pub fn success(message: &str) {
    println!("{}", format_log(LogLevel::Info, &format!("✓ {}", message)));
}

/// Log a progress message (shown without newline for updates)
pub fn progress(message: &str) {
    if should_log(LogLevel::Info) {
        print!(
            "\r[{}] [{}] {}",
            Local::now().format("%H:%M:%S"),
            "PROG",
            message
        );
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
}

/// Log a step in a process
pub fn step(step_num: usize, total: usize, message: &str) {
    if should_log(LogLevel::Info) {
        println!(
            "[{}] [STEP {}/{}] {}",
            Local::now().format("%H:%M:%S"),
            step_num,
            total,
            message
        );
    }
}

/// Log a separator line
pub fn separator() {
    if should_log(LogLevel::Info) {
        println!("{}", "─".repeat(60));
    }
}

/// Log a header
pub fn header(title: &str) {
    if should_log(LogLevel::Info) {
        println!();
        println!("{}", "═".repeat(60));
        println!("  {}", title);
        println!("{}", "═".repeat(60));
    }
}

/// Log a sub-item (indented)
pub fn sub_item(message: &str) {
    if should_log(LogLevel::Info) {
        println!("    {}", message);
    }
}

/// Log a summary section
pub fn summary(title: &str, items: &[(&str, String)]) {
    if should_log(LogLevel::Info) {
        println!();
        println!("[{}] [SUMMARY] {}", Local::now().format("%H:%M:%S"), title);
        for (key, value) in items {
            println!("    {}: {}", key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("INFO"), LogLevel::Info);
        assert_eq!(LogLevel::from_str("unknown"), LogLevel::Info);
    }
}
