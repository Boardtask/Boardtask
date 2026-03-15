use askama::Template;
use axum::{
    extract::State,
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
    #[allow(dead_code)]
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

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/app/teams", get(list))
}
