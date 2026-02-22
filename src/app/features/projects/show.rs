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

use super::progress;

fn format_estimated_minutes(minutes: i64) -> String {
    if minutes == 0 {
        "—".to_string()
    } else if minutes < 60 {
        format!("{} min", minutes)
    } else {
        let h = minutes / 60;
        let m = minutes % 60;
        if m == 0 {
            format!("{} h", h)
        } else {
            format!("{} h {} min", h, m)
        }
    }
}

/// Project detail template.
#[derive(Template)]
#[template(path = "projects_show.html")]
pub struct ProjectShowTemplate {
    pub app_name: &'static str,
    pub project: db::projects::Project,
    pub todo_count: i64,
    pub in_progress_count: i64,
    pub completed_count: i64,
    pub total_count: i64,
    pub blocked_count: i64,
    pub blocked_todo_count: i64,
    pub blocked_in_progress_count: i64,
    pub total_estimated_display: String,
    pub estimated_left_display: String,
    pub estimated_completed_display: String,
}

/// GET /app/projects/:id — Show project detail.
pub async fn show(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let project = match db::projects::find_by_id(&state.db, &id).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "Project not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    // Validate org membership - never trust session, validate against resource's org
    if tenant::require_org_member(&state.db, &session.user_id, &project.organization_id)
        .await
        .is_err()
    {
        return (StatusCode::NOT_FOUND, "Project not found").into_response();
    }

    let (nodes, edges) = match tokio::try_join!(
        db::nodes::find_by_project(&state.db, &id),
        db::node_edges::find_by_project(&state.db, &id),
    ) {
        Ok((n, e)) => (n, e),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let group_ids: std::collections::HashSet<&str> =
        nodes.iter().filter_map(|n| n.parent_id.as_deref()).collect();
    let task_nodes: Vec<&db::nodes::Node> = nodes
        .iter()
        .filter(|n| !group_ids.contains(n.id.as_str()))
        .collect();

    let total_count = task_nodes.len() as i64;
    let todo_count = task_nodes
        .iter()
        .filter(|n| n.status_id == db::task_statuses::TODO_STATUS_ID)
        .count() as i64;
    let in_progress_count = task_nodes
        .iter()
        .filter(|n| n.status_id == db::task_statuses::IN_PROGRESS_STATUS_ID)
        .count() as i64;
    let completed_count = task_nodes
        .iter()
        .filter(|n| n.status_id == db::task_statuses::DONE_STATUS_ID)
        .count() as i64;

    let (blocked_count, blocked_todo_count, blocked_in_progress_count) =
        progress::count_blocked(&nodes, &edges);

    let total_estimated_minutes: i64 = task_nodes.iter().filter_map(|n| n.estimated_minutes).sum();
    let estimated_completed_minutes: i64 = task_nodes
        .iter()
        .filter(|n| n.status_id == db::task_statuses::DONE_STATUS_ID)
        .filter_map(|n| n.estimated_minutes)
        .sum();
    let estimated_left_minutes = total_estimated_minutes.saturating_sub(estimated_completed_minutes);

    let total_estimated_display = format_estimated_minutes(total_estimated_minutes);
    let estimated_left_display = format_estimated_minutes(estimated_left_minutes);
    let estimated_completed_display = format_estimated_minutes(estimated_completed_minutes);

    ProjectShowTemplate {
        app_name: APP_NAME,
        project,
        todo_count,
        in_progress_count,
        completed_count,
        total_count,
        blocked_count,
        blocked_todo_count,
        blocked_in_progress_count,
        total_estimated_display,
        estimated_left_display,
        estimated_completed_display,
    }
    .into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/app/projects/:id", get(show))
}
