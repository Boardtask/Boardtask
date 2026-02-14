use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::delete,
    Router,
};

use crate::app::{
    db,
    error::AppError,
    session::ApiAuthenticatedSession,
    AppState,
};

/// DELETE /api/projects/:project_id/nodes/:id — Delete a node.
pub async fn delete_node(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(params): Path<super::types::NodePathParams>,
) -> Result<StatusCode, AppError> {
    // Ensure user owns the project
    let _project = super::helpers::ensure_project_owned(&state.db, &params.project_id, &session.user_id).await?;

    // Load the existing node
    let node = db::nodes::find_by_id(&state.db, &params.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Node not found".to_string()))?;

    // Verify node belongs to the project
    if node.project_id != params.project_id {
        return Err(AppError::NotFound("Node not found".to_string()));
    }

    // Delete the node
    db::nodes::delete(&state.db, &node.id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Node deletion routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/projects/:project_id/nodes/:id", delete(delete_node))
}