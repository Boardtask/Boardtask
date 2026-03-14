use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

/// Per-project default view mode when opening from the projects list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ProjectViewMode {
    Graph,
    List,
}

impl Default for ProjectViewMode {
    fn default() -> Self {
        Self::Graph
    }
}
