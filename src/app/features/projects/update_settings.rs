use axum::{
    extract::{Path, State},
    routing::patch,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::app::{
    db,
    domain::ProjectViewMode,
    error::AppError,
    features::graph,
    session::ApiAuthenticatedSession,
    AppState,
};

/// Request body for PATCH /api/projects/:id (update project settings).
#[derive(Debug, Deserialize)]
pub struct UpdateProjectSettingsRequest {
    pub default_view_mode: Option<ProjectViewMode>,
}

/// Response for project settings update.
#[derive(Debug, Serialize)]
pub struct UpdateProjectSettingsResponse {
    pub default_view_mode: String,
}

/// PATCH /api/projects/:id — Update project settings (e.g. default_view_mode).
pub async fn patch_project(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(request): Json<UpdateProjectSettingsRequest>,
) -> Result<Json<UpdateProjectSettingsResponse>, AppError> {
    let mode = request
        .default_view_mode
        .ok_or_else(|| AppError::Validation("default_view_mode is required".to_string()))?;

    let project = graph::helpers::ensure_project_accessible(&state.db, &project_id, &session.user_id).await?;

    let updated = db::projects::update_default_view_mode(
        &state.db,
        &project_id,
        &project.organization_id,
        mode,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

    Ok(Json(UpdateProjectSettingsResponse {
        default_view_mode: updated.default_view_mode().to_string(),
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/projects/:id", patch(patch_project))
}