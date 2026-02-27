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

use super::{format, helpers, progress};

/// Project detail template.
#[derive(Template)]
#[template(path = "projects_show.html")]
pub struct ProjectShowTemplate {
    pub app_name: &'static str,
    pub project: db::projects::Project,
    pub todo_count: i64,
    pub in_progress_count: i64,
    pub completed_count: i64,
    pub blocked_count: i64,
    pub estimated_left_display: String,
}

/// GET /app/projects/:id — Show project detail.
pub async fn show(
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

    let (nodes, edges) = match tokio::try_join!(
        db::nodes::find_by_project(&state.db, &id),
        db::node_edges::find_by_project(&state.db, &id),
    ) {
        Ok((n, e)) => (n, e),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let task_nodes = helpers::task_nodes_from_nodes(&nodes);

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

    let (blocked_count, _, _) = progress::count_blocked(&nodes, &edges);

    let total_estimated_minutes: i64 =
        task_nodes.iter().filter_map(|n| n.estimated_minutes).sum();
    let estimated_completed_minutes: i64 = task_nodes
        .iter()
        .filter(|n| n.status_id == db::task_statuses::DONE_STATUS_ID)
        .filter_map(|n| n.estimated_minutes)
        .sum();
    let estimated_left_minutes = total_estimated_minutes.saturating_sub(estimated_completed_minutes);

    let estimated_left_display =
        format::format_estimated_minutes(estimated_left_minutes);

    ProjectShowTemplate {
        app_name: APP_NAME,
        project,
        todo_count,
        in_progress_count,
        completed_count,
        blocked_count,
        estimated_left_display,
    }
    .into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/app/projects/:id", get(show))
}
