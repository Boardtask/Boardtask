use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::app::{
    db,
    error::AppError,
    session::ApiAuthenticatedSession,
    AppState,
};

/// Request body for creating an edge.
#[derive(Debug, Deserialize)]
pub struct CreateEdgeRequest {
    pub parent_id: String,
    pub child_id: String,
}

/// Response for a created edge.
#[derive(Debug, Serialize)]
pub struct EdgeResponse {
    pub parent_id: String,
    pub child_id: String,
    pub created_at: i64,
}

/// POST /api/projects/:project_id/edges — Create a new edge.
pub async fn create_edge(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(request): Json<CreateEdgeRequest>,
) -> Result<(StatusCode, Json<EdgeResponse>), AppError> {
    // Ensure user owns the project
    let _project = super::helpers::ensure_project_owned(&state.db, &project_id, &session.user_id).await?;

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

    // Reject self-referencing edges
    if request.parent_id == request.child_id {
        return Err(AppError::Validation("Cannot create edge from node to itself".to_string()));
    }

    // Create and insert edge
    let new_edge = db::node_edges::NewNodeEdge {
        parent_id: request.parent_id.clone(),
        child_id: request.child_id.clone(),
    };

    db::node_edges::insert(&state.db, &new_edge).await?;

    // Fetch the created edge for response
    let edge = db::node_edges::find_by_project(&state.db, &project_id)
        .await?
        .into_iter()
        .find(|e| e.parent_id == request.parent_id && e.child_id == request.child_id)
        .ok_or_else(|| AppError::Internal)?;

    let response = EdgeResponse {
        parent_id: edge.parent_id,
        child_id: edge.child_id,
        created_at: edge.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Edge creation routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/projects/:project_id/edges", post(create_edge))
}