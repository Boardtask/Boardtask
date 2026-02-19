use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::delete,
    Json, Router,
};
use serde::Deserialize;

use crate::app::{
    db,
    error::AppError,
    session::ApiAuthenticatedSession,
    AppState,
};

/// Request body for creating an edge (reused for delete).
#[derive(Debug, Deserialize)]
pub struct CreateEdgeRequest {
    pub parent_id: String,
    pub child_id: String,
}

/// DELETE /api/projects/:project_id/edges — Delete an edge.
pub async fn delete_edge(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(request): Json<CreateEdgeRequest>,
) -> Result<StatusCode, AppError> {
    // Validate org membership on every write
    let _project = super::helpers::ensure_project_accessible(&state.db, &project_id, &session.user_id).await?;

    // Validate both nodes exist and belong to the project
    let parent_node = db::nodes::find_by_id(&state.db, &request.parent_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Parent node not found".to_string()))?;

    let child_node = db::nodes::find_by_id(&state.db, &request.child_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Child node not found".to_string()))?;

    if parent_node.project_id != project_id {
        return Err(AppError::NotFound("Parent node not found".to_string()));
    }

    if child_node.project_id != project_id {
        return Err(AppError::NotFound("Child node not found".to_string()));
    }

    // Delete the edge (idempotent - succeeds even if edge doesn't exist)
    db::node_edges::delete(&state.db, &request.parent_id, &request.child_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Edge deletion routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/projects/:project_id/edges", delete(delete_edge))
}