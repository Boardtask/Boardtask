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

/// Derive phase label from progress percentage (for progress column).
fn phase_label_from_progress(progress_percent: i32) -> &'static str {
    match progress_percent {
        0..=24 => "Planning",
        25..=49 => "Scaling Phase",
        50..=74 => "Security Audit",
        75..=99 => "Beta Deployment",
        _ => "Complete",
    }
}

/// One row for the projects list table.
pub struct ProjectRow {
    pub id: String,
    pub title: String,
    pub description: String,
    pub progress_percent: i32,
    pub phase_label: &'static str,
    pub node_count: i64,
    pub blocker_count: i64,
    pub team_avatar_urls: Vec<String>,
    pub team_overflow: i64,
}

/// Projects list template.
#[derive(Template)]
#[template(path = "projects_list.html")]
pub struct ProjectsListTemplate {
    pub app_name: &'static str,
    pub projects: Vec<ProjectRow>,
    pub current_user_avatar_url: String,
    pub total_tasks: i64,
    pub total_blockers: i64,
    pub contributor_count: i64,
    pub global_efficiency_display: String,
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

    let user_id = match crate::app::domain::UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid session".to_string()).into_response(),
    };
    let current_user_avatar_url =
        db::users::profile_image_url_for(&state.db, &user_id).await;

    // Org members count for footer "Contributors"
    let org_id = match crate::app::domain::OrganizationId::from_string(&session.organization_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid organization".to_string()).into_response(),
    };
    let org_members = db::organizations::list_members_with_email(&state.db, &org_id)
        .await
        .unwrap_or_default();
    let contributor_count = org_members.len() as i64;

    let mut projects = Vec::new();
    let mut total_tasks: i64 = 0;
    let mut total_blockers: i64 = 0;
    let mut total_completed: i64 = 0;

    for p in db_projects {
        let node_count = db::nodes::count_by_project(&state.db, &p.id).await.unwrap_or(0);
        let completed_count = db::nodes::count_by_project_and_status(
            &state.db,
            &p.id,
            db::task_statuses::DONE_STATUS_ID,
        )
        .await
        .unwrap_or(0);
        let progress_percent = if node_count > 0 {
            ((completed_count * 100) / node_count) as i32
        } else {
            0
        };

        // Blocker count requires nodes + edges
        let (nodes, edges) = match tokio::try_join!(
            db::nodes::find_by_project(&state.db, &p.id),
            db::node_edges::find_by_project(&state.db, &p.id),
        ) {
            Ok((n, e)) => (n, e),
            Err(_) => (Vec::new(), Vec::new()),
        };
        let (blocker_count, _, _) = super::progress::count_blocked(&nodes, &edges);

        total_tasks += node_count;
        total_blockers += blocker_count;
        total_completed += completed_count;

        // Team avatars (from project's team)
        let (team_avatar_urls, team_overflow) = if let Some(ref team_id) = p.team_id {
            let urls = db::team_members::list_avatar_urls_for_team(&state.db, team_id)
                .await
                .unwrap_or_default();
            let total = db::team_members::count_by_team(&state.db, team_id)
                .await
                .unwrap_or(0);
            let overflow = (total - urls.len() as i64).max(0);
            (urls, overflow)
        } else {
            (Vec::new(), 0)
        };

        projects.push(ProjectRow {
            id: p.id,
            title: p.title,
            description: String::new(), // Projects don't have description
            progress_percent,
            phase_label: phase_label_from_progress(progress_percent),
            node_count,
            blocker_count,
            team_avatar_urls,
            team_overflow,
        });
    }

    let global_efficiency_display = if total_tasks > 0 {
        format!("{:.1}", (total_completed as f64 / total_tasks as f64) * 100.0)
    } else {
        "0.0".to_string()
    };

    ProjectsListTemplate {
        app_name: APP_NAME,
        projects,
        current_user_avatar_url,
        total_tasks,
        total_blockers,
        contributor_count,
        global_efficiency_display,
    }
    .into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/app/projects", get(list))
}
