//! Configuration constants for the Mapper.

use std::path::PathBuf;

/// Campus information (URL, name)
pub const CAMPUSES: &[(&str, &str)] = &[
    ("https://www.yonsei.ac.kr/sc/186/subview.do", "신촌캠퍼스"),
    (
        "https://mirae.yonsei.ac.kr/wj/1413/subview.do",
        "미래캠퍼스",
    ),
];

/// Board keyword mappings
pub struct KeywordMeta {
    pub id: &'static str,
    pub name: &'static str,
}

pub const KEYWORD_MAP: &[(&str, KeywordMeta)] = &[
    (
        "학부공지",
        KeywordMeta {
            id: "academic",
            name: "학사공지",
        },
    ),
    (
        "대학원공지",
        KeywordMeta {
            id: "grad_notice",
            name: "대학원공지",
        },
    ),
    (
        "장학",
        KeywordMeta {
            id: "scholarship",
            name: "장학공지",
        },
    ),
    (
        "취업",
        KeywordMeta {
            id: "career",
            name: "취업/진로",
        },
    ),
    (
        "공지사항",
        KeywordMeta {
            id: "notice",
            name: "일반공지",
        },
    ),
    (
        "학사공지",
        KeywordMeta {
            id: "academic",
            name: "학사공지",
        },
    ),
];

/// HTTP request timeout in seconds
pub const REQUEST_TIMEOUT_SECS: u64 = 10;

/// User agent string
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// Get the data directory path
pub fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

/// Output file paths
pub fn departments_file() -> PathBuf {
    data_dir().join("yonsei_departments.json")
}

pub fn departments_boards_file() -> PathBuf {
    data_dir().join("yonsei_departments_boards.json")
}

pub fn manual_review_file() -> PathBuf {
    data_dir().join("manual_review_needed.json")
}
