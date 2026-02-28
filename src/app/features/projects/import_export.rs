//! Shared DTOs for project JSON import/export. One schema for both directions.

use serde::{Deserialize, Serialize};

/// Current format version for project export/import.
pub const EXPORT_VERSION: u32 = 1;

/// Top-level project export/import payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectExport {
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exported_at: Option<String>,
    pub project: ProjectExportProject,
    #[serde(default)]
    pub slots: Vec<ProjectExportSlot>,
    #[serde(default)]
    pub nodes: Vec<ProjectExportNode>,
    #[serde(default)]
    pub edges: Vec<ProjectExportEdge>,
}

/// Project metadata (title only; id/org/team assigned on import).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectExportProject {
    pub title: String,
}

/// Slot in export; id used only for mapping to nodes on import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectExportSlot {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
}

/// Node in export; id used for edges/parent mapping. No project_id or timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectExportNode {
    pub id: String,
    pub node_type_id: String,
    pub status_id: String,
    pub title: String,
    pub description: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub slot_id: Option<String>,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub assigned_user_id: Option<String>,
}

/// Edge in export; parent_id and child_id refer to node ids in the same payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectExportEdge {
    pub parent_id: String,
    pub child_id: String,
}
