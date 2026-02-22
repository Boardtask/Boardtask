use axum::http::StatusCode;

use crate::app::db;

/// Load project by id. Returns the project or an (status, message) for HTML error responses.
/// Caller is responsible for auth (e.g. tenant::require_org_member).
pub async fn load_project(
    pool: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<db::projects::Project, (StatusCode, &'static str)> {
    match db::projects::find_by_id(pool, project_id).await {
        Ok(Some(p)) => Ok(p),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Project not found")),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Database error")),
    }
}

/// Return references to task-only nodes: nodes that are not a parent of any other (i.e. exclude group nodes).
pub fn task_nodes_from_nodes<'a>(nodes: &'a [db::nodes::Node]) -> Vec<&'a db::nodes::Node> {
    let group_ids: std::collections::HashSet<&str> =
        nodes.iter().filter_map(|n| n.parent_id.as_deref()).collect();
    nodes
        .iter()
        .filter(|n| !group_ids.contains(n.id.as_str()))
        .collect()
}
