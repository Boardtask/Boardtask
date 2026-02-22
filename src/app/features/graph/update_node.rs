use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::patch,
    Json, Router,
};
use serde::{
    de::Deserializer,
    Deserialize,
    Serialize,
};
use validator::Validate;

use crate::app::{
    db,
    error::AppError,
    session::ApiAuthenticatedSession,
    AppState,
};

/// Deserializes a JSON value so that missing key => None, present null => Some(None), present value => Some(Some(v)).
/// Required to distinguish "omit field" (leave unchanged) from "field: null" (clear estimate).
fn deserialize_optional_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Request body for updating a node (partial update).
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateNodeRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    #[validate(length(max = 2000))]
    pub description: Option<String>,
    pub node_type_id: Option<String>,
    pub status_id: Option<String>,
    /// Omit = unchanged, null = clear slot.
    #[serde(default, deserialize_with = "deserialize_optional_option")]
    pub slot_id: Option<Option<String>>,
    /// Omit = unchanged, null = clear parent (ungroup).
    #[serde(default, deserialize_with = "deserialize_optional_option")]
    pub parent_id: Option<Option<String>>,
    /// Omit = unchanged, null = clear estimate.
    #[serde(default, deserialize_with = "deserialize_optional_option")]
    #[validate(custom(function = "crate::app::features::graph::helpers::validate_estimated_minutes"))]
    pub estimated_minutes: Option<Option<i64>>,
}

/// Response for an updated node.
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
    pub slot_id: Option<String>,
    pub parent_id: Option<String>,
}

/// Validates update-node request (sync rules + DB-backed node_type_id, status_id, slot_id, parent_id when provided).
async fn validate_update_node_request(
    request: &UpdateNodeRequest,
    pool: &sqlx::SqlitePool,
    merged_node_type_id: &str,
    merged_status_id: &str,
    _merged_slot_id: &Option<String>,
    _merged_parent_id: &Option<String>,
    project_id: &str,
    node_id: &str,
) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|_| AppError::Validation("Invalid input".to_string()))?;

    if request.node_type_id.is_some() {
        let _ = db::node_types::find_by_id(pool, merged_node_type_id)
            .await?
            .ok_or_else(|| AppError::Validation("Invalid node_type_id".to_string()))?;
    }
    if request.status_id.is_some() {
        let _ = db::task_statuses::find_by_id(pool, merged_status_id)
            .await?
            .ok_or_else(|| AppError::Validation("Invalid status_id".to_string()))?;
    }
    if let Some(Some(slot_id)) = &request.slot_id {
        let slot = db::project_slots::find_by_id(pool, slot_id)
            .await?
            .ok_or_else(|| AppError::Validation("Invalid slot_id".to_string()))?;
        if slot.project_id != project_id {
            return Err(AppError::Validation("Invalid slot_id".to_string()));
        }
    }
    if let Some(Some(pid)) = &request.parent_id {
        if pid == node_id {
            return Err(AppError::Validation("parent_id cannot be self".to_string()));
        }
        let parent = db::nodes::find_by_id(pool, pid)
            .await?
            .ok_or_else(|| AppError::Validation("Invalid parent_id".to_string()))?;
        if parent.project_id != project_id {
            return Err(AppError::Validation("Invalid parent_id".to_string()));
        }
    }

    Ok(())
}

/// PATCH /api/projects/:project_id/nodes/:id — Update a node.
pub async fn update_node(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(params): Path<super::types::NodePathParams>,
    Json(request): Json<UpdateNodeRequest>,
) -> Result<(StatusCode, Json<NodeResponse>), AppError> {
    // Validate org membership on every write
    let _project = super::helpers::ensure_project_accessible(&state.db, &params.project_id, &session.user_id).await?;

    let node = db::nodes::find_by_id(&state.db, &params.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Node not found".to_string()))?;

    if node.project_id != params.project_id {
        return Err(AppError::NotFound("Node not found".to_string()));
    }

    let node_type_id = request.node_type_id.as_deref().unwrap_or(&node.node_type_id);
    let status_id = request.status_id.as_deref().unwrap_or(&node.status_id);
    let slot_id = request
        .slot_id
        .clone()
        .unwrap_or_else(|| node.slot_id.clone());
    let parent_id = request
        .parent_id
        .clone()
        .unwrap_or_else(|| node.parent_id.clone());
    validate_update_node_request(
        &request,
        &state.db,
        node_type_id,
        status_id,
        &slot_id,
        &parent_id,
        &params.project_id,
        &node.id,
    )
    .await?;

    let title = request.title.as_deref().unwrap_or(&node.title);
    let description = request.description.or(node.description);
    let estimated_minutes = request.estimated_minutes.unwrap_or(node.estimated_minutes);

    db::nodes::update(
        &state.db,
        &node.id,
        title,
        description.as_deref(),
        node_type_id,
        status_id,
        estimated_minutes,
        slot_id.as_deref(),
        parent_id.as_deref(),
    )
    .await?;

    // Fetch the updated node for response
    let updated_node = db::nodes::find_by_id(&state.db, &node.id)
        .await?
        .ok_or_else(|| AppError::Internal)?;

    let response = NodeResponse {
        id: updated_node.id,
        project_id: updated_node.project_id,
        node_type_id: updated_node.node_type_id,
        status_id: updated_node.status_id,
        title: updated_node.title,
        description: updated_node.description,
        created_at: updated_node.created_at,
        updated_at: updated_node.updated_at,
        estimated_minutes: updated_node.estimated_minutes,
        slot_id: updated_node.slot_id,
        parent_id: updated_node.parent_id,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Node update routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/projects/:project_id/nodes/:id", patch(update_node))
}