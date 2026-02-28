//! GET /api/projects/:project_id/members — List org members for the project's organization (for assignee dropdown).

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::app::{
    db,
    error::AppError,
    session::ApiAuthenticatedSession,
    AppState,
};

/// Member row for assignee dropdown (user_id, display name, email, profile image).
#[derive(Debug, Serialize)]
pub struct ProjectMemberItem {
    pub user_id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub profile_image_url: Option<String>,
}

/// Response for listing project org members.
#[derive(Debug, Serialize)]
pub struct ProjectMembersResponse {
    pub members: Vec<ProjectMemberItem>,
}

/// GET /api/projects/:project_id/members — List members of the project's organization.
pub async fn get_project_members(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectMembersResponse>, AppError> {
    let project = super::helpers::ensure_project_accessible(&state.db, &project_id, &session.user_id).await?;
    let org_id = crate::app::domain::OrganizationId::from_string(&project.organization_id)
        .map_err(|_| AppError::NotFound("Project not found".to_string()))?;

    let rows = db::organizations::list_members_with_email(&state.db, &org_id).await?;
    let members: Vec<ProjectMemberItem> = rows
        .into_iter()
        .map(|r| ProjectMemberItem {
            user_id: r.user_id,
            email: r.email,
            first_name: r.first_name,
            last_name: r.last_name,
            profile_image_url: r.profile_image_url,
        })
        .collect();

    Ok(Json(ProjectMembersResponse { members }))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/projects/:project_id/members", get(get_project_members))
}
