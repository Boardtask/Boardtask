use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use ulid::Ulid;
use validator::Validate;

use crate::app::{
    db,
    error::AppError,
    session::ApiAuthenticatedSession,
    AppState,
};

/// Request body for creating a node.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateNodeRequest {
    pub node_type_id: String,
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    #[validate(length(max = 2000))]
    pub description: Option<String>,
    pub status_id: Option<String>,
    #[validate(custom(function = "crate::app::features::graph::helpers::validate_estimated_minutes"))]
    pub estimated_minutes: Option<i64>,
}

/// Response for a created node.
#[derive(Debug, Serialize)]
pub struct NodeResponse {
    pub id: String,
    pub project_id: String,
    pub node_type_id: String,
    pub status_id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub estimated_minutes: Option<i64>,
}

/// Validates create-node request (sync rules + DB-backed node_type_id and status_id). Returns (node_type_id, status_id).
async fn validate_create_node_request(
    request: &CreateNodeRequest,
    pool: &sqlx::SqlitePool,
) -> Result<(String, String), AppError> {
    request
        .validate()
        .map_err(|_| AppError::Validation("Invalid input".to_string()))?;

    let _ = db::node_types::find_by_id(pool, &request.node_type_id)
        .await?
        .ok_or_else(|| AppError::Validation("Invalid node_type_id".to_string()))?;

    let status_id = match &request.status_id {
        None => super::helpers::DEFAULT_STATUS_ID.to_string(),
        Some(s) => {
            let _ = db::task_statuses::find_by_id(pool, s)
                .await?
                .ok_or_else(|| AppError::Validation("Invalid status_id".to_string()))?;
            s.clone()
        }
    };

    Ok((request.node_type_id.clone(), status_id))
}

/// POST /api/projects/:project_id/nodes — Create a new node.
pub async fn create_node(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(request): Json<CreateNodeRequest>,
) -> Result<(StatusCode, Json<NodeResponse>), AppError> {
    // Validate org membership on every write
    let project = super::helpers::ensure_project_accessible(&state.db, &project_id, &session.user_id).await?;

    let (node_type_id, status_id) = validate_create_node_request(&request, &state.db).await?;

    // Generate ULID for node
    let node_id = Ulid::new().to_string();

    // Create and insert node
    let new_node = db::nodes::NewNode {
        id: node_id.clone(),
        project_id: project.id.clone(),
        node_type_id,
        status_id,
        title: request.title,
        description: request.description,
        estimated_minutes: request.estimated_minutes,
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
        status_id: node.status_id,
        title: node.title,
        description: node.description,
        created_at: node.created_at,
        updated_at: node.updated_at,
        estimated_minutes: node.estimated_minutes,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Node creation routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/projects/:project_id/nodes", post(create_node))
}