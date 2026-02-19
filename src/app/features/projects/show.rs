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

/// Project detail template.
#[derive(Template)]
#[template(path = "projects_show.html")]
pub struct ProjectShowTemplate {
    pub app_name: &'static str,
    pub project: db::projects::Project,
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

    ProjectShowTemplate {
        app_name: APP_NAME,
        project,
    }
    .into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/app/projects/:id", get(show))
}
