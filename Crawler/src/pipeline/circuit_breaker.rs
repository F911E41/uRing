//! Circuit Breaker pattern implementation.
//!
//! Prevents data corruption by aborting writes when notice count drops
//! significantly compared to the previous run.
//!
//! ## Specification
//!
//! > If the number of crawled items drops by more than **20%** compared
//! > to the previous run, the write operation is aborted.

use crate::error::{AppError, Result};
use crate::models::NoticeOutput;

/// Circuit breaker configuration.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Maximum allowed drop percentage (0-100). Default: 20%
    pub max_drop_percent: u8,
    /// Minimum notice count to trigger circuit breaker check.
    /// Below this threshold, the check is skipped (for new deployments).
    pub min_baseline: usize,
    /// Allow empty results when previous was also empty
    pub allow_cold_start: bool,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            max_drop_percent: 20,
            min_baseline: 10,
            allow_cold_start: true,
        }
    }
}

/// Circuit breaker for preventing bad data updates.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
}

/// Result of circuit breaker check.
#[derive(Debug, Clone)]
pub enum CircuitBreakerResult {
    /// Safe to proceed with the write
    Safe {
        current_count: usize,
        previous_count: usize,
    },
    /// First run or cold start - no previous data
    ColdStart { current_count: usize },
    /// Circuit breaker triggered - abort write
    Triggered {
        current_count: usize,
        previous_count: usize,
        drop_percent: f64,
    },
    /// Empty result - critical failure
    EmptyResult,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default configuration.
    pub fn new() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom configuration.
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self { config }
    }

    /// Check if it's safe to write the new notices.
    ///
    /// Returns `CircuitBreakerResult` indicating whether to proceed.
    pub fn check(
        &self,
        current: &[NoticeOutput],
        previous: &[NoticeOutput],
    ) -> CircuitBreakerResult {
        let current_count = current.len();
        let previous_count = previous.len();

        // Case 1: Empty current result
        if current_count == 0 {
            if previous_count == 0 && self.config.allow_cold_start {
                return CircuitBreakerResult::ColdStart { current_count };
            }
            return CircuitBreakerResult::EmptyResult;
        }

        // Case 2: Cold start (no previous data or below baseline)
        if previous_count < self.config.min_baseline {
            return CircuitBreakerResult::ColdStart { current_count };
        }

        // Case 3: Check drop percentage
        if current_count < previous_count {
            let drop = previous_count - current_count;
            let drop_percent = (drop as f64 / previous_count as f64) * 100.0;

            if drop_percent > self.config.max_drop_percent as f64 {
                return CircuitBreakerResult::Triggered {
                    current_count,
                    previous_count,
                    drop_percent,
                };
            }
        }

        // Safe to proceed
        CircuitBreakerResult::Safe {
            current_count,
            previous_count,
        }
    }

    /// Validate and return Ok if safe, Err if circuit breaker triggered.
    pub fn validate(&self, current: &[NoticeOutput], previous: &[NoticeOutput]) -> Result<()> {
        match self.check(current, previous) {
            CircuitBreakerResult::Safe {
                current_count,
                previous_count,
            } => {
                log::info!(
                    "Circuit breaker: SAFE ({} notices, was {})",
                    current_count,
                    previous_count
                );
                Ok(())
            }
            CircuitBreakerResult::ColdStart { current_count } => {
                log::info!(
                    "Circuit breaker: COLD START ({} notices, first run or below baseline)",
                    current_count
                );
                Ok(())
            }
            CircuitBreakerResult::Triggered {
                current_count,
                previous_count,
                drop_percent,
            } => {
                log::error!(
                    "Circuit breaker: TRIGGERED! {} → {} notices ({:.1}% drop > {}% threshold)",
                    previous_count,
                    current_count,
                    drop_percent,
                    self.config.max_drop_percent
                );
                Err(AppError::CircuitBreakerTriggered {
                    current_count,
                    previous_count,
                    drop_percent,
                    threshold_percent: self.config.max_drop_percent,
                })
            }
            CircuitBreakerResult::EmptyResult => {
                log::error!("Circuit breaker: EMPTY RESULT - aborting write");
                Err(AppError::EmptyCrawlResult)
            }
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NoticeMetadata;

    fn make_notices(count: usize) -> Vec<NoticeOutput> {
        (0..count)
            .map(|i| NoticeOutput {
                id: format!("notice_{}", i),
                title: format!("Notice {}", i),
                link: format!("https://example.com/{}", i),
                metadata: NoticeMetadata {
                    campus: "Test".into(),
                    college: "".into(),
                    department_name: "Dept".into(),
                    board_name: "Board".into(),
                    date: "2026-02-02".into(),
                    pinned: false,
                },
            })
            .collect()
    }

    #[test]
    fn test_safe_no_drop() {
        let cb = CircuitBreaker::new();
        let current = make_notices(100);
        let previous = make_notices(100);

        assert!(matches!(
            cb.check(&current, &previous),
            CircuitBreakerResult::Safe { .. }
        ));
    }

    #[test]
    fn test_safe_small_drop() {
        let cb = CircuitBreaker::new();
        let current = make_notices(85); // 15% drop
        let previous = make_notices(100);

        assert!(matches!(
            cb.check(&current, &previous),
            CircuitBreakerResult::Safe { .. }
        ));
    }

    #[test]
    fn test_triggered_large_drop() {
        let cb = CircuitBreaker::new();
        let current = make_notices(70); // 30% drop
        let previous = make_notices(100);

        assert!(matches!(
            cb.check(&current, &previous),
            CircuitBreakerResult::Triggered { .. }
        ));
    }

    #[test]
    fn test_cold_start() {
        let cb = CircuitBreaker::new();
        let current = make_notices(50);
        let previous = vec![]; // No previous data

        assert!(matches!(
            cb.check(&current, &previous),
            CircuitBreakerResult::ColdStart { .. }
        ));
    }

    #[test]
    fn test_empty_result() {
        let cb = CircuitBreaker::new();
        let current = vec![];
        let previous = make_notices(100);

        assert!(matches!(
            cb.check(&current, &previous),
            CircuitBreakerResult::EmptyResult
        ));
    }

    #[test]
    fn test_increase_is_safe() {
        let cb = CircuitBreaker::new();
        let current = make_notices(150); // 50% increase
        let previous = make_notices(100);

        assert!(matches!(
            cb.check(&current, &previous),
            CircuitBreakerResult::Safe { .. }
        ));
    }

    #[test]
    fn test_validate_returns_error() {
        let cb = CircuitBreaker::new();
        let current = make_notices(50); // 50% drop
        let previous = make_notices(100);

        let result = cb.validate(&current, &previous);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AppError::CircuitBreakerTriggered { .. }
        ));
    }
}
