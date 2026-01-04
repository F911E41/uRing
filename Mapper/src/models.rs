//! Data models for the Mapper.

use serde::{Deserialize, Serialize};

use crate::selectors::CmsSelectors;

/// Represents a notice board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(flatten)]
    pub selectors: CmsSelectors,
}

/// Represents a university department
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Department {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub boards: Vec<Board>,
}

/// Represents a college containing departments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct College {
    pub name: String,
    pub departments: Vec<Department>,
}

/// Represents a university campus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campus {
    pub campus: String,
    pub colleges: Vec<College>,
}

/// Represents a department that needs manual review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualReviewItem {
    pub campus: String,
    pub name: String,
    pub url: String,
    pub reason: String,
}

/// Result of board discovery for a department
#[derive(Debug, Default)]
pub struct BoardDiscoveryResult {
    pub boards: Vec<Board>,
    pub manual_review: Option<ManualReviewItem>,
}
