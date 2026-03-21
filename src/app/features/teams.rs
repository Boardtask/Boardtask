use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

use crate::app::{
    db,
    session::AuthenticatedSession,
    tenant,
    AppState, APP_NAME,
};

/// One row for the teams list table.
pub(crate) struct TeamRow {
    pub id: String,
    pub name: String,
    pub member_count: i64,
    pub project_count: i64,
}

/// Teams list template.
#[derive(Template)]
#[template(path = "teams_list.html")]
pub(crate) struct TeamsListTemplate {
    pub app_name: &'static str,
    pub teams: Vec<TeamRow>,
    pub current_user_avatar_url: String,
}

/// One row for the team members table on the team detail page.
pub(crate) struct TeamMemberRow {
    pub display_name: String,
    pub email: String,
    pub avatar_url: String,
    pub initials: String,
}

/// Team detail template (members list).
#[derive(Template)]
#[template(path = "teams_show.html")]
pub(crate) struct TeamsShowTemplate {
    pub app_name: &'static str,
    pub team_name: String,
    pub project_count: i64,
    pub members: Vec<TeamMemberRow>,
    pub members_total: usize,
    pub current_user_avatar_url: String,
}

fn initials_from_name(name: &str, email: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() >= 2 {
        let first = parts[0].chars().next().unwrap_or('?');
        let last = parts[1].chars().next().unwrap_or('?');
        format!(
            "{}{}",
            first.to_uppercase().collect::<String>(),
            last.to_uppercase().collect::<String>()
        )
    } else if !name.is_empty() {
        name.chars().take(2).flat_map(|c| c.to_uppercase()).collect()
    } else {
        email.chars().take(2).flat_map(|c| c.to_uppercase()).collect()
    }
}

/// GET /app/teams — List org's teams (scoped by org membership).
pub(crate) async fn list(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if tenant::require_org_member(&state.db, &session.user_id, &session.organization_id)
        .await
        .is_err()
    {
        return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response();
    }

    let db_teams = match db::teams::find_by_organization(&state.db, &session.organization_id).await
    {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let user_id = match crate::app::domain::UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid session".to_string()).into_response()
        }
    };
    let current_user_avatar_url =
        db::users::profile_image_url_for(&state.db, &user_id).await;

    let mut teams = Vec::new();
    for team in db_teams {
        let member_count = db::team_members::count_by_team(&state.db, &team.id)
            .await
            .unwrap_or(0);
        let project_count = db::projects::count_by_team(&state.db, &team.id)
            .await
            .unwrap_or(0);
        teams.push(TeamRow {
            id: team.id,
            name: team.name,
            member_count,
            project_count,
        });
    }

    let template = TeamsListTemplate {
        app_name: APP_NAME,
        teams,
        current_user_avatar_url,
    };
    Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response()
}

/// GET /app/teams/:team_id — Team detail and members (tenant-scoped by team's org).
pub(crate) async fn show(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Path(team_id): Path<String>,
) -> impl IntoResponse {
    let user_id = match crate::app::domain::UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid session".to_string()).into_response()
        }
    };

    let team = match db::teams::find_by_id(&state.db, &team_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    if tenant::require_org_member(&state.db, &session.user_id, &team.organization_id)
        .await
        .is_err()
    {
        return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response();
    }

    let current_user_avatar_url =
        db::users::profile_image_url_for(&state.db, &user_id).await;

    let project_count = db::projects::count_by_team(&state.db, &team_id)
        .await
        .unwrap_or(0);

    let rows = match db::team_members::list_members_with_user_details(&state.db, &team_id).await {
        Ok(r) => r,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let members: Vec<TeamMemberRow> = rows
        .into_iter()
        .map(|m| {
            let display_name = db::users::display_name_from_parts(&m.first_name, &m.last_name);
            let display_name = if display_name.is_empty() {
                m.email.clone()
            } else {
                display_name
            };
            let initials = initials_from_name(&display_name, &m.email);
            let avatar_url = m.profile_image_url.unwrap_or_default();
            TeamMemberRow {
                display_name,
                email: m.email,
                avatar_url,
                initials,
            }
        })
        .collect();
    let members_total = members.len();

    let template = TeamsShowTemplate {
        app_name: APP_NAME,
        team_name: team.name,
        project_count,
        members,
        members_total,
        current_user_avatar_url,
    };
    Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response()
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/app/teams", get(list))
        .route("/app/teams/:team_id", get(show))
}
