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

    let total_count = match db::nodes::count_by_project(&state.db, &id).await {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };
    let todo_count = match db::nodes::count_by_project_and_status(
        &state.db,
        &id,
        db::task_statuses::TODO_STATUS_ID,
    )
    .await
    {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };
    let in_progress_count = match db::nodes::count_by_project_and_status(
        &state.db,
        &id,
        db::task_statuses::IN_PROGRESS_STATUS_ID,
    )
    .await
    {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };
    let completed_count = match db::nodes::count_by_project_and_status(
        &state.db,
        &id,
        db::task_statuses::DONE_STATUS_ID,
    )
    .await
    {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let (nodes, edges) = match tokio::try_join!(
        db::nodes::find_by_project(&state.db, &id),
        db::node_edges::find_by_project(&state.db, &id),
    ) {
        Ok((n, e)) => (n, e),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };
    let (blocked_count, blocked_todo_count, blocked_in_progress_count) =
        progress::count_blocked(&nodes, &edges);

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
    }
    .into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/app/projects/:id", get(show))
}
