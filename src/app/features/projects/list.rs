use askama::Template;
use axum::{
    extract::State,
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

/// Projects list template.
#[derive(Template)]
#[template(path = "projects_list.html")]
pub struct ProjectsListTemplate {
    pub app_name: &'static str,
    pub projects: Vec<db::projects::Project>,
}

/// GET /app/projects — List user's projects (scoped by org).
pub async fn list(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Validate org membership - scope every read
    if tenant::require_org_member(&state.db, &session.user_id, &session.organization_id)
        .await
        .is_err()
    {
        return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response();
    }

    let projects = match db::projects::find_by_user_and_org(
        &state.db,
        &session.user_id,
        &session.organization_id,
    )
    .await
    {
        Ok(p) => p,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    ProjectsListTemplate {
        app_name: APP_NAME,
        projects,
    }
    .into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/app/projects", get(list))
}
