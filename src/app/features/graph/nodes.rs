use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::app::{
    db,
    error::AppError,
    session::ApiAuthenticatedSession,
    AppState,
};

/// Request body for creating a node.
#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub node_type_id: String,
    pub title: String,
    pub description: Option<String>,
}

/// Response for a created node.
#[derive(Debug, Serialize)]
pub struct NodeResponse {
    pub id: String,
    pub project_id: String,
    pub node_type_id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: Option<i64>,
}

/// POST /app/api/projects/:project_id/nodes — Create a new node.
pub async fn create_node(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(request): Json<CreateNodeRequest>,
) -> Result<(StatusCode, Json<NodeResponse>), AppError> {
    // Ensure user owns the project
    let project = ensure_project_owned(&state.db, &project_id, &session.user_id).await?;

    // Validate node_type_id exists
    let _node_type = db::node_types::find_by_id(&state.db, &request.node_type_id)
        .await?
        .ok_or_else(|| AppError::Validation("Invalid node_type_id".to_string()))?;

    // Validate title length (1-255)
    if request.title.is_empty() || request.title.len() > 255 {
        return Err(AppError::Validation("Title must be 1-255 characters".to_string()));
    }

    // Generate ULID for node
    let node_id = Ulid::new().to_string();

    // Create and insert node
    let new_node = db::nodes::NewNode {
        id: node_id.clone(),
        project_id: project.id.clone(),
        node_type_id: request.node_type_id,
        title: request.title,
        description: request.description,
    };

    db::nodes::insert(&state.db, &new_node).await?;

    // Fetch the created node for response
    let node = db::nodes::find_by_id(&state.db, &node_id)
        .await?
        .ok_or_else(|| AppError::Internal)?;

    let response = NodeResponse {
        id: node.id,
        project_id: node.project_id,
        node_type_id: node.node_type_id,
        title: node.title,
        description: node.description,
        created_at: node.created_at,
        updated_at: node.updated_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Helper to ensure a user owns a project, returning NotFound if not found or not owned.
async fn ensure_project_owned(
    pool: &sqlx::SqlitePool,
    project_id: &str,
    user_id: &str,
) -> Result<db::projects::Project, AppError> {
    let project = db::projects::find_by_id(pool, project_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

    if project.user_id != user_id {
        return Err(AppError::NotFound("Project not found".to_string()));
    }

    Ok(project)
}

/// Node API routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/projects/:project_id/nodes", post(create_node))
}