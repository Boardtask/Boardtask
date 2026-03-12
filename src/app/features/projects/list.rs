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

/// One row for the projects list table (id, title, formatted created date, team, creator, task counts, progress).
pub struct ProjectRow {
    pub id: String,
    pub title: String,
    pub created_at_display: String,
    pub team_name: String,
    pub creator_name: String,
    pub node_count: i64,
    pub completed_count: i64,
    pub progress_percent: i32,
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

    let mut projects = Vec::new();

    for p in db_projects {
        let created_at_display = OffsetDateTime::from_unix_timestamp(p.created_at)
            .map(|dt| dt.date().to_string())
            .unwrap_or_else(|_| "—".into());

        // Resolve team name
        let team_name = if let Some(team_id) = p.team_id {
            if let Ok(Some(team)) = db::teams::find_by_id(&state.db, &team_id).await {
                team.name
            } else {
                "—".to_string()
            }
        } else {
            "—".to_string()
        };

        // Resolve creator name
        let creator_name = if let Ok(Some(user)) = db::users::find_by_id(&state.db, &crate::app::domain::UserId::from_string(&p.user_id).unwrap()).await {
            db::users::display_name(&user)
        } else {
            "—".to_string()
        };

        // Count nodes and completed nodes
        let node_count = db::nodes::count_by_project(&state.db, &p.id).await.unwrap_or(0);
        let completed_count = db::nodes::count_by_project_and_status(&state.db, &p.id, db::task_statuses::DONE_STATUS_ID).await.unwrap_or(0);
        let progress_percent = if node_count > 0 {
            ((completed_count * 100) / node_count) as i32
        } else {
            0
        };

        projects.push(ProjectRow {
            id: p.id,
            title: p.title,
            created_at_display,
            team_name,
            creator_name,
            node_count,
            completed_count,
            progress_percent,
        });
    }

    ProjectsListTemplate {
        app_name: APP_NAME,
        projects,
    }
    .into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/app/projects", get(list))
}
