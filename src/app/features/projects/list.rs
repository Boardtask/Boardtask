use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use time::OffsetDateTime;

use crate::app::{
    db,
    session::AuthenticatedSession,
    tenant,
    AppState, APP_NAME,
};

/// One row for the projects list table (id, title, formatted created date).
pub struct ProjectRow {
    pub id: String,
    pub title: String,
    pub created_at_display: String,
}

/// Projects list template.
#[derive(Template)]
#[template(path = "projects_list.html")]
pub struct ProjectsListTemplate {
    pub app_name: &'static str,
    pub projects: Vec<ProjectRow>,
}

/// GET /app/projects — List org's projects (scoped by org membership).
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

    let db_projects = match db::projects::list_for_org(&state.db, &session.organization_id).await {
        Ok(p) => p,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let projects: Vec<ProjectRow> = db_projects
        .into_iter()
        .map(|p| {
            let created_at_display = OffsetDateTime::from_unix_timestamp(p.created_at)
                .map(|dt| dt.date().to_string())
                .unwrap_or_else(|_| "—".into());
            ProjectRow {
                id: p.id,
                title: p.title,
                created_at_display,
            }
        })
        .collect();

    ProjectsListTemplate {
        app_name: APP_NAME,
        projects,
    }
    .into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/app/projects", get(list))
}
