//! Inverted Index generation for serverless search.
//!
//! Builds a static inverted index mapping keywords to notice IDs,
//! enabling client-side full-text search without a backend search engine.
//!
//! > During the crawl, an **Inverted Index** (`index.json`) is generated,
//! > mapping keywords to Announcement IDs.
//! >
//! > Example: `{"scholarship": ["id_001", "id_005"], "dorm": ["id_002"]}`

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

use crate::models::NoticeOutput;

/// Configuration for index generation.
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Minimum token length to include (default: 2)
    pub min_token_length: usize,
    /// Maximum tokens per notice (default: 50)
    pub max_tokens_per_notice: usize,
    /// Include metadata fields in indexing (campus, department, board)
    pub index_metadata: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            min_token_length: 2,
            max_tokens_per_notice: 50,
            index_metadata: true,
        }
    }
}

/// Inverted index for full-text search.
///
/// Maps normalized keywords to sets of notice IDs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InvertedIndex {
    /// Version for cache busting
    pub version: u32,
    /// Total number of indexed notices
    pub notice_count: usize,
    /// Total number of unique tokens
    pub token_count: usize,
    /// The inverted index: keyword -> list of notice IDs
    pub index: HashMap<String, Vec<String>>,
}

/// Builder for constructing an inverted index.
pub struct IndexBuilder {
    config: IndexConfig,
    index: HashMap<String, HashSet<String>>,
    notice_count: usize,
}

impl IndexBuilder {
    /// Create a new index builder with default configuration.
    pub fn new() -> Self {
        Self::with_config(IndexConfig::default())
    }

    /// Create a new index builder with custom configuration.
    pub fn with_config(config: IndexConfig) -> Self {
        Self {
            config,
            index: HashMap::new(),
            notice_count: 0,
        }
    }

    /// Add a notice to the index.
    pub fn add_notice(&mut self, notice: &NoticeOutput) {
        self.notice_count += 1;
        let id = &notice.id;

        // Tokenize title
        let mut tokens = self.tokenize(&notice.title);

        // Optionally tokenize metadata
        if self.config.index_metadata {
            tokens.extend(self.tokenize(&notice.metadata.campus));
            tokens.extend(self.tokenize(&notice.metadata.department_name));
            tokens.extend(self.tokenize(&notice.metadata.board_name));
            if !notice.metadata.college.is_empty() {
                tokens.extend(self.tokenize(&notice.metadata.college));
            }
        }

        // Limit tokens per notice
        tokens.truncate(self.config.max_tokens_per_notice);

        // Add to index
        for token in tokens {
            self.index.entry(token).or_default().insert(id.clone());
        }
    }

    /// Add multiple notices to the index.
    pub fn add_notices(&mut self, notices: &[NoticeOutput]) {
        for notice in notices {
            self.add_notice(notice);
        }
    }

    /// Build the final inverted index.
    pub fn build(self) -> InvertedIndex {
        let token_count = self.index.len();
        let index: HashMap<String, Vec<String>> = self
            .index
            .into_iter()
            .map(|(k, v)| {
                let mut ids: Vec<_> = v.into_iter().collect();
                ids.sort(); // Deterministic output
                (k, ids)
            })
            .collect();

        InvertedIndex {
            version: 1,
            notice_count: self.notice_count,
            token_count,
            index,
        }
    }

    /// Tokenize a string into normalized keywords.
    fn tokenize(&self, text: &str) -> Vec<String> {
        let normalized = text.to_lowercase();

        // Use unicode-aware word segmentation
        normalized
            .unicode_words()
            .filter(|word| word.len() >= self.config.min_token_length)
            .filter(|word| !is_stopword(word))
            .map(String::from)
            .collect()
    }
}

impl Default for IndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a word is a common stopword (Korean/English).
fn is_stopword(word: &str) -> bool {
    const STOPWORDS: &[&str] = &[
        // Korean common particles/endings
        "및", "의", "를", "을", "가", "이", "은", "는", "에서", "으로", "로", "와", "과",
        // English common words
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "can", "must",
        "shall", "of", "to", "in", "for", "on", "with", "at", "by", "from", "as", "or", "and",
        "but", "if", "then", "so", "than", // Common URL/HTML artifacts
        "http", "https", "www", "com", "kr", "html", "php", "asp",
    ];
    STOPWORDS.contains(&word)
}

/// Build an inverted index from a list of notices.
pub fn build_index(notices: &[NoticeOutput]) -> InvertedIndex {
    let mut builder = IndexBuilder::new();
    builder.add_notices(notices);
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NoticeMetadata;

    fn sample_notice(id: &str, title: &str) -> NoticeOutput {
        NoticeOutput {
            id: id.to_string(),
            title: title.to_string(),
            link: format!("https://example.com/{}", id),
            metadata: NoticeMetadata {
                campus: "신촌캠퍼스".into(),
                college: "공과대학".into(),
                department_name: "컴퓨터공학과".into(),
                board_name: "학사공지".into(),
                date: "2026-02-02".into(),
                pinned: false,
            },
        }
    }

    #[test]
    fn test_build_index() {
        let notices = vec![
            sample_notice("001", "장학금 신청 안내"),
            sample_notice("002", "기숙사 입사 신청"),
            sample_notice("003", "장학금 수령 방법"),
        ];

        let index = build_index(&notices);

        assert_eq!(index.notice_count, 3);
        assert!(index.token_count > 0);

        // "장학금" should map to notices 001 and 003
        let scholarship_ids = index.index.get("장학금");
        assert!(scholarship_ids.is_some());
        let ids = scholarship_ids.unwrap();
        assert!(ids.contains(&"001".to_string()));
        assert!(ids.contains(&"003".to_string()));
        assert!(!ids.contains(&"002".to_string()));
    }

    #[test]
    fn test_metadata_indexing() {
        let notices = vec![sample_notice("001", "공지사항")];
        let index = build_index(&notices);

        // Should include metadata tokens
        assert!(index.index.contains_key("신촌캠퍼스"));
        assert!(index.index.contains_key("컴퓨터공학과"));
    }

    #[test]
    fn test_stopword_filtering() {
        let notices = vec![sample_notice("001", "the quick brown fox")];
        let index = build_index(&notices);

        // "the" should be filtered out
        assert!(!index.index.contains_key("the"));
        // "quick", "brown", "fox" should be present
        assert!(index.index.contains_key("quick"));
        assert!(index.index.contains_key("brown"));
        assert!(index.index.contains_key("fox"));
    }

    #[test]
    fn test_min_token_length() {
        let notices = vec![sample_notice("001", "a b cd efg")];
        let index = build_index(&notices);

        // Single-character tokens should be filtered
        assert!(!index.index.contains_key("a"));
        assert!(!index.index.contains_key("b"));
        // "cd" and "efg" should be present
        assert!(index.index.contains_key("cd"));
        assert!(index.index.contains_key("efg"));
    }
}
