//! Diff calculation for Event-Driven Notifications.
//!
//! Computes the difference between two snapshots to identify
//! new, updated, and removed notices for notification dispatch.
//!
//! > The Notifier calculates the **Diff** between the new and old versions.
//! > If new items are detected, push notifications are dispatched via FCM.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::models::{Diff, NoticeOutput};

/// Extended diff result with full notice data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiffResult {
    /// Basic diff (IDs only)
    pub diff: Diff,
    /// Full notice objects for added notices (for notifications)
    pub added_notices: Vec<NoticeOutput>,
    /// Full notice objects for updated notices
    pub updated_notices: Vec<NoticeOutput>,
}

impl DiffResult {
    /// Check if there are any changes.
    pub fn has_changes(&self) -> bool {
        !self.diff.added.is_empty()
            || !self.diff.updated.is_empty()
            || !self.diff.removed.is_empty()
    }

    /// Get the total number of changes.
    pub fn change_count(&self) -> usize {
        self.diff.added.len() + self.diff.updated.len() + self.diff.removed.len()
    }
}

/// Calculator for computing diffs between snapshots.
#[derive(Debug, Clone, Default)]
pub struct DiffCalculator {
    /// Whether to detect updates (title changes for same ID)
    detect_updates: bool,
}

impl DiffCalculator {
    /// Create a new diff calculator.
    pub fn new() -> Self {
        Self {
            detect_updates: true,
        }
    }

    /// Create a diff calculator that only detects additions/removals.
    pub fn additions_only() -> Self {
        Self {
            detect_updates: false,
        }
    }

    /// Calculate the diff between previous and current snapshots.
    pub fn calculate(&self, previous: &[NoticeOutput], current: &[NoticeOutput]) -> DiffResult {
        let prev_map: HashMap<&str, &NoticeOutput> =
            previous.iter().map(|n| (n.id.as_str(), n)).collect();

        let curr_map: HashMap<&str, &NoticeOutput> =
            current.iter().map(|n| (n.id.as_str(), n)).collect();

        let prev_ids: HashSet<&str> = prev_map.keys().copied().collect();
        let curr_ids: HashSet<&str> = curr_map.keys().copied().collect();

        // Added: in current but not in previous
        let added_ids: Vec<String> = curr_ids
            .difference(&prev_ids)
            .map(|id| id.to_string())
            .collect();

        let added_notices: Vec<NoticeOutput> = added_ids
            .iter()
            .filter_map(|id| curr_map.get(id.as_str()).copied().cloned())
            .collect();

        // Removed: in previous but not in current
        let removed: Vec<String> = prev_ids
            .difference(&curr_ids)
            .map(|id| id.to_string())
            .collect();

        // Updated: in both but title changed
        let (updated, updated_notices) = if self.detect_updates {
            let common: Vec<&str> = prev_ids.intersection(&curr_ids).copied().collect();
            let mut updated_ids = Vec::new();
            let mut updated_notices = Vec::new();

            for id in common {
                let prev = prev_map.get(id).unwrap();
                let curr = curr_map.get(id).unwrap();

                // Check if title changed (could expand to other fields)
                if prev.title != curr.title {
                    updated_ids.push(id.to_string());
                    updated_notices.push((*curr).clone());
                }
            }
            (updated_ids, updated_notices)
        } else {
            (Vec::new(), Vec::new())
        };

        DiffResult {
            diff: Diff {
                added: added_ids,
                updated,
                removed,
            },
            added_notices,
            updated_notices,
        }
    }
}

/// Convenience function to calculate diff.
pub fn calculate_diff(previous: &[NoticeOutput], current: &[NoticeOutput]) -> DiffResult {
    DiffCalculator::new().calculate(previous, current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NoticeMetadata;

    fn make_notice(id: &str, title: &str) -> NoticeOutput {
        NoticeOutput {
            id: id.to_string(),
            title: title.to_string(),
            link: format!("https://example.com/{}", id),
            metadata: NoticeMetadata {
                campus: "Test".into(),
                college: "".into(),
                department_name: "Dept".into(),
                board_name: "Board".into(),
                date: "2026-02-02".into(),
                pinned: false,
            },
        }
    }

    #[test]
    fn test_no_changes() {
        let prev = vec![make_notice("001", "Title 1"), make_notice("002", "Title 2")];
        let curr = prev.clone();

        let result = calculate_diff(&prev, &curr);
        assert!(!result.has_changes());
        assert_eq!(result.change_count(), 0);
    }

    #[test]
    fn test_additions() {
        let prev = vec![make_notice("001", "Title 1")];
        let curr = vec![
            make_notice("001", "Title 1"),
            make_notice("002", "Title 2"),
            make_notice("003", "Title 3"),
        ];

        let result = calculate_diff(&prev, &curr);
        assert!(result.has_changes());
        assert_eq!(result.diff.added.len(), 2);
        assert!(result.diff.added.contains(&"002".to_string()));
        assert!(result.diff.added.contains(&"003".to_string()));
        assert_eq!(result.added_notices.len(), 2);
    }

    #[test]
    fn test_removals() {
        let prev = vec![make_notice("001", "Title 1"), make_notice("002", "Title 2")];
        let curr = vec![make_notice("001", "Title 1")];

        let result = calculate_diff(&prev, &curr);
        assert!(result.has_changes());
        assert_eq!(result.diff.removed.len(), 1);
        assert!(result.diff.removed.contains(&"002".to_string()));
    }

    #[test]
    fn test_updates() {
        let prev = vec![make_notice("001", "Old Title")];
        let curr = vec![make_notice("001", "New Title")];

        let result = calculate_diff(&prev, &curr);
        assert!(result.has_changes());
        assert_eq!(result.diff.updated.len(), 1);
        assert!(result.diff.updated.contains(&"001".to_string()));
        assert_eq!(result.updated_notices[0].title, "New Title");
    }

    #[test]
    fn test_mixed_changes() {
        let prev = vec![
            make_notice("001", "Keep"),
            make_notice("002", "Update Me"),
            make_notice("003", "Remove Me"),
        ];
        let curr = vec![
            make_notice("001", "Keep"),
            make_notice("002", "Updated"),
            make_notice("004", "New Notice"),
        ];

        let result = calculate_diff(&prev, &curr);
        assert_eq!(result.diff.added, vec!["004"]);
        assert_eq!(result.diff.updated, vec!["002"]);
        assert_eq!(result.diff.removed, vec!["003"]);
    }

    #[test]
    fn test_empty_to_full() {
        let prev: Vec<NoticeOutput> = vec![];
        let curr = vec![make_notice("001", "First Notice")];

        let result = calculate_diff(&prev, &curr);
        assert_eq!(result.diff.added.len(), 1);
        assert!(result.diff.removed.is_empty());
    }

    #[test]
    fn test_full_to_empty() {
        let prev = vec![make_notice("001", "Last Notice")];
        let curr: Vec<NoticeOutput> = vec![];

        let result = calculate_diff(&prev, &curr);
        assert!(result.diff.added.is_empty());
        assert_eq!(result.diff.removed.len(), 1);
    }
}
