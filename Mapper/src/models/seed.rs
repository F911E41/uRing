//! Seed data model structures (campuses, keywords, CMS patterns).

use serde::Deserialize;

/// Root seed data structure
#[derive(Debug, Deserialize, Clone)]
pub struct Seed {
    pub campuses: Vec<CampusInfo>,
    pub keywords: Vec<KeywordMapping>,
    #[serde(default)]
    pub cms_patterns: Vec<CmsPattern>,
}

/// Campus information
#[derive(Debug, Deserialize, Clone)]
pub struct CampusInfo {
    pub name: String,
    pub url: String,
}

/// Board keyword to ID mapping
#[derive(Debug, Deserialize, Clone)]
pub struct KeywordMapping {
    pub keyword: String,
    pub id: String,
    pub display_name: String,
}

/// CMS detection pattern and selectors
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct CmsPattern {
    pub name: String,
    #[serde(default)]
    pub detect_url_contains: Option<String>,
    #[serde(default)]
    pub detect_html_contains: Option<String>,
    pub row_selector: String,
    pub title_selector: String,
    pub date_selector: String,
    pub link_attr: String,
}

impl Default for Seed {
    fn default() -> Self {
        Seed {
            campuses: vec![
                CampusInfo {
                    name: "신촌캠퍼스".to_string(),
                    url: "https://www.yonsei.ac.kr/sc/186/subview.do".to_string(),
                },
                CampusInfo {
                    name: "미래캠퍼스".to_string(),
                    url: "https://mirae.yonsei.ac.kr/wj/1413/subview.do".to_string(),
                },
            ],
            keywords: vec![
                KeywordMapping {
                    keyword: "학부공지".to_string(),
                    id: "academic".to_string(),
                    display_name: "학사공지".to_string(),
                },
                KeywordMapping {
                    keyword: "학사공지".to_string(),
                    id: "academic".to_string(),
                    display_name: "학사공지".to_string(),
                },
                KeywordMapping {
                    keyword: "대학원공지".to_string(),
                    id: "grad_notice".to_string(),
                    display_name: "대학원공지".to_string(),
                },
                KeywordMapping {
                    keyword: "장학".to_string(),
                    id: "scholarship".to_string(),
                    display_name: "장학공지".to_string(),
                },
                KeywordMapping {
                    keyword: "취업".to_string(),
                    id: "career".to_string(),
                    display_name: "취업/진로".to_string(),
                },
                KeywordMapping {
                    keyword: "공지사항".to_string(),
                    id: "notice".to_string(),
                    display_name: "일반공지".to_string(),
                },
            ],
            cms_patterns: vec![
                CmsPattern {
                    name: "yonsei_standard".to_string(),
                    detect_url_contains: Some(".do".to_string()),
                    detect_html_contains: Some("c-board-title".to_string()),
                    row_selector: "tr:has(a.c-board-title)".to_string(),
                    title_selector: "a.c-board-title".to_string(),
                    date_selector: "td:nth-last-child(1)".to_string(),
                    link_attr: "href".to_string(),
                },
                CmsPattern {
                    name: "xe_board".to_string(),
                    detect_url_contains: None,
                    detect_html_contains: Some("xe-list-board".to_string()),
                    row_selector: "li.xe-list-board-list--item:not(.xe-list-board-list--header)"
                        .to_string(),
                    title_selector: "a.xe-list-board-list__title-link".to_string(),
                    date_selector: ".xe-list-board-list__created_at".to_string(),
                    link_attr: "href".to_string(),
                },
            ],
        }
    }
}
