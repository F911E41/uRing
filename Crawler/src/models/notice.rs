//! Notice data structure.

use serde::{Deserialize, Serialize};

/// A notice fetched from a board.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notice {
    /// Campus name
    pub campus: String,

    /// College name (empty string if department is directly under campus)
    pub college: String,

    /// Department unique identifier
    pub department_id: String,

    /// Department display name
    pub department_name: String,

    /// Board unique identifier
    pub board_id: String,

    /// Board display name
    pub board_name: String,

    /// Notice title
    pub title: String,

    /// Notice date
    pub date: String,

    /// Full URL to the notice
    pub link: String,
}

impl Notice {
    /// Format notice for display using a template.
    ///
    /// Supported placeholders:
    /// - `{campus}`, `{college}`, `{dept_id}`, `{dept_name}`
    /// - `{board_id}`, `{board_name}`, `{title}`, `{date}`, `{link}`
    pub fn format(&self, template: &str) -> String {
        template
            .replace("{campus}", &self.campus)
            .replace("{college}", &self.college)
            .replace("{dept_id}", &self.department_id)
            .replace("{dept_name}", &self.department_name)
            .replace("{board_id}", &self.board_id)
            .replace("{board_name}", &self.board_name)
            .replace("{title}", &self.title)
            .replace("{date}", &self.date)
            .replace("{link}", &self.link)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_notice() -> Notice {
        Notice {
            campus: "TestCampus".to_string(),
            college: "TestCollege".to_string(),
            department_id: "dept1".to_string(),
            department_name: "Department".to_string(),
            board_id: "notice".to_string(),
            board_name: "공지사항".to_string(),
            title: "Test Title".to_string(),
            date: "2024-01-01".to_string(),
            link: "https://example.com/notice/1".to_string(),
        }
    }

    #[test]
    fn test_format() {
        let notice = sample_notice();
        let result = notice.format("[{dept_name}] {title}");
        assert_eq!(result, "[Department] Test Title");
    }
}
