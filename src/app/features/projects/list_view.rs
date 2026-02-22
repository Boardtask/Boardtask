use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};

use crate::app::{
    db,
    session::AuthenticatedSession,
    tenant,
    AppState, APP_NAME,
};

use super::{format, helpers};

/// One task row for the list view (node plus resolved display names and formatted estimate).
#[derive(Clone)]
pub struct TaskRow {
    pub node: db::nodes::Node,
    pub node_type_name: String,
    pub status_name: String,
    pub slot_name: String,
    pub estimated_display: String,
}

/// Project list view template (tasks only, no group nodes).
#[derive(Template)]
#[template(path = "projects_list_view.html")]
pub struct ProjectListViewTemplate {
    pub app_name: &'static str,
    pub project: db::projects::Project,
    pub task_rows: Vec<TaskRow>,
    pub task_rows_json: String,
}

/// GET /app/projects/:id/list — List view of project tasks.
pub async fn list_view(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let project = match helpers::load_project(&state.db, &id).await {
        Ok(p) => p,
        Err((status, msg)) => return (status, msg).into_response(),
    };
    if tenant::require_org_member(&state.db, &session.user_id, &project.organization_id)
        .await
        .is_err()
    {
        return (StatusCode::NOT_FOUND, "Project not found").into_response();
    }

    let (nodes, node_types, task_statuses, slots) = match tokio::try_join!(
        db::nodes::find_by_project(&state.db, &id),
        db::node_types::get_all_systems(&state.db),
        db::task_statuses::get_all_task_statuses(&state.db),
        db::project_slots::find_by_project(&state.db, &id),
    ) {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let task_nodes = helpers::task_nodes_from_nodes(&nodes);

    let type_by_id: std::collections::HashMap<&str, &db::node_types::NodeType> = node_types
        .iter()
        .map(|t| (t.id.as_str(), t))
        .collect();
    let status_by_id: std::collections::HashMap<&str, &db::task_statuses::TaskStatus> =
        task_statuses.iter().map(|s| (s.id.as_str(), s)).collect();
    let slot_by_id: std::collections::HashMap<&str, &db::project_slots::ProjectSlot> = slots
        .iter()
        .map(|s| (s.id.as_str(), s))
        .collect();

    let task_rows: Vec<TaskRow> = task_nodes
        .into_iter()
        .map(|n| {
            let node = n.clone();
            let node_type_name = type_by_id
                .get(n.node_type_id.as_str())
                .map(|t| t.name.as_str())
                .unwrap_or("Task")
                .to_string();
            let status_name = status_by_id
                .get(n.status_id.as_str())
                .map(|s| s.name.as_str())
                .unwrap_or("To do")
                .to_string();
            let slot_name = n
                .slot_id
                .as_ref()
                .and_then(|sid| slot_by_id.get(sid.as_str()))
                .map(|s| s.name.as_str())
                .unwrap_or("")
                .to_string();
            let estimated_display = n
                .estimated_minutes
                .map(format::format_estimated_minutes)
                .unwrap_or_else(|| "—".to_string());
            TaskRow {
                node,
                node_type_name,
                status_name,
                slot_name,
                estimated_display,
            }
        })
        .collect();

    let task_rows_json = serde_json::to_string(
        &task_rows
            .iter()
            .map(|r| &r.node)
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string());

    ProjectListViewTemplate {
        app_name: APP_NAME,
        project,
        task_rows,
        task_rows_json,
    }
    .into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/app/projects/:id/list", get(list_view))
}
