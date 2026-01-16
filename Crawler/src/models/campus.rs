// src/models/campus.rs

//! Campus, College, Department, and Board data structures.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::CmsSelectors;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampusMeta {
    pub id: String,
    pub name: String,
}

impl From<&Campus> for CampusMeta {
    fn from(campus: &Campus) -> Self {
        Self {
            id: campus.campus.clone(), // Assuming the 'campus' field is the ID
            name: campus.campus.clone(),
        }
    }
}

/// A university campus containing colleges and/or departments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campus {
    /// Campus name (e.g., "신촌캠퍼스", "미래캠퍼스")
    pub campus: String,

    /// Colleges within this campus
    #[serde(default)]
    pub colleges: Vec<College>,

    /// Departments directly under campus (without college)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub departments: Vec<Department>,
}

impl Campus {
    /// Load campus configurations from a JSON file.
    pub fn load_all(path: impl AsRef<Path>) -> Result<Vec<Self>> {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Get all departments with their hierarchical context.
    pub fn all_departments(&self) -> Vec<DepartmentRef<'_>> {
        let mut result = Vec::new();

        // Departments within colleges
        for college in &self.colleges {
            for dept in &college.departments {
                result.push(DepartmentRef {
                    campus: &self.campus,
                    college: Some(&college.name),
                    dept,
                });
            }
        }

        // Departments directly under campus
        for dept in &self.departments {
            result.push(DepartmentRef {
                campus: &self.campus,
                college: None,
                dept,
            });
        }

        result
    }

    /// Count total departments in this campus.
    pub fn department_count(&self) -> usize {
        self.colleges
            .iter()
            .map(|c| c.departments.len())
            .sum::<usize>()
            + self.departments.len()
    }

    /// Count total boards in this campus.
    pub fn board_count(&self) -> usize {
        self.colleges
            .iter()
            .flat_map(|c| &c.departments)
            .chain(&self.departments)
            .map(|d| d.boards.len())
            .sum()
    }
}

/// Reference to a department with its hierarchical context.
#[derive(Debug, Clone, Copy)]
pub struct DepartmentRef<'a> {
    pub campus: &'a str,
    pub college: Option<&'a str>,
    pub dept: &'a Department,
}

/// A college containing multiple departments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct College {
    pub name: String,
    pub departments: Vec<Department>,
}

/// A university department with its notice boards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Department {
    /// Unique identifier for the department
    pub id: String,

    /// Display name
    pub name: String,

    /// Department homepage URL
    pub url: String,

    /// Notice boards discovered for this department
    #[serde(default)]
    pub boards: Vec<Board>,
}

/// A notice board within a department.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    /// Unique identifier for the board
    pub id: String,

    /// Display name (e.g., "학사공지", "장학공지")
    pub name: String,

    /// URL of the board listing page
    pub url: String,

    /// CSS selectors for scraping
    #[serde(flatten)]
    pub selectors: CmsSelectors,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_campus() -> Campus {
        Campus {
            campus: "TestCampus".to_string(),
            colleges: vec![College {
                name: "TestCollege".to_string(),
                departments: vec![Department {
                    id: "dept1".to_string(),
                    name: "Department 1".to_string(),
                    url: "https://example.com".to_string(),
                    boards: vec![],
                }],
            }],
            departments: vec![],
        }
    }

    #[test]
    fn test_all_departments() {
        let campus = create_test_campus();
        let deps = campus.all_departments();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].college, Some("TestCollege"));
    }

    #[test]
    fn test_department_count() {
        let campus = create_test_campus();
        assert_eq!(campus.department_count(), 1);
    }
}
