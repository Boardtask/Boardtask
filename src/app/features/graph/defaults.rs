use sqlx::SqlitePool;

use crate::app::db;

/// System node type defaults: (id, name, color).
const SYSTEM_NODE_TYPES: &[(&str, &str, &str)] = &[
    ("01JNODETYPE00000000TASK000", "Task", "#3B82F6"),
    ("01JNODETYPE00000000BUG0000", "Bug", "#EF4444"),
    ("01JNODETYPE00000000EPIC000", "Epic", "#8B5CF6"),
    ("01JNODETYPE00000000MILESTON", "Milestone", "#F59E0B"),
    ("01JNODETYPE00000000SPIKE00", "Spike", "#10B981"),
    ("01JNODETYPE00000000STORY00", "Story", "#06B6D4"),
];

/// Sync system node types. Idempotent — skips if already exists.
pub async fn sync_system_node_types(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for (id, name, color) in SYSTEM_NODE_TYPES {
        if db::node_types::find_by_id(pool, id).await?.is_some() {
            continue;
        }
        let node_type = db::node_types::NewNodeType {
            id: id.to_string(),
            user_id: None,
            name: name.to_string(),
            color: color.to_string(),
        };
        db::node_types::insert(pool, &node_type).await?;
    }
    Ok(())
}