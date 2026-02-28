use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use ulid::Ulid;
use validator::Validate;

use crate::app::{
    db,
    domain::{OrganizationId, UserId},
    error::AppError,
    session::ApiAuthenticatedSession,
    AppState,
};

/// Request body for inserting a node between an existing edge's parent and child.
#[derive(Debug, Deserialize, Validate)]
pub struct InsertBetweenRequest {
    /// Existing parent in the edge A -> B.
    pub parent_id: String,
    /// Existing child in the edge A -> B.
    pub child_id: String,
    /// Node type for the new node.
    pub node_type_id: String,
    /// Title for the new node.
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    /// Optional description for the new node.
    #[validate(length(max = 2000))]
    pub description: Option<String>,
    /// Optional explicit status for the new node; defaults to system "To do".
    pub status_id: Option<String>,
    /// Optional slot assignment for the new node.
    pub slot_id: Option<String>,
    /// Optional assignee (user) for the new node. Must be org member.
    pub assigned_user_id: Option<String>,
    /// Optional grouping parent (compound/group node id) for the new node.
    pub group_id: Option<String>,
}

/// POST /api/projects/:project_id/edges/insert-between — Insert a node between two connected nodes.
///
/// This operation is transactional: it creates the new node and rewires the edge
/// from A -> B into A -> C and C -> B in a single database transaction.
pub async fn insert_between(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(request): Json<InsertBetweenRequest>,
) -> Result<(StatusCode, Json<crate::app::features::graph::create_node::NodeResponse>), AppError> {
    // Validate org membership on every write and load project (tenant isolation).
    let project =
        super::helpers::ensure_project_accessible(&state.db, &project_id, &session.user_id)
            .await?;

    // Basic request validation (title/description length).
    request
        .validate()
        .map_err(|_| AppError::Validation("Invalid input".to_string()))?;

    // Reject self-referential edges early.
    if request.parent_id == request.child_id {
        return Err(AppError::Validation(
            "Cannot insert between a self-referential edge".to_string(),
        ));
    }

    // Validate node_type_id exists.
    let _ = db::node_types::find_by_id(&state.db, &request.node_type_id)
        .await?
        .ok_or_else(|| AppError::Validation("Invalid node_type_id".to_string()))?;

    // Resolve status_id (default to system status when omitted).
    let status_id = match &request.status_id {
        None => super::helpers::DEFAULT_STATUS_ID.to_string(),
        Some(s) => {
            let _ = db::task_statuses::find_by_id(&state.db, s)
                .await?
                .ok_or_else(|| AppError::Validation("Invalid status_id".to_string()))?;
            s.clone()
        }
    };

    // Validate slot_id belongs to the project when provided.
    let slot_id = match &request.slot_id {
        None => None,
        Some(s) => {
            let slot = db::project_slots::find_by_id(&state.db, s)
                .await?
                .ok_or_else(|| AppError::Validation("Invalid slot_id".to_string()))?;
            if slot.project_id != project_id {
                return Err(AppError::Validation("Invalid slot_id".to_string()));
            }
            Some(s.clone())
        }
    };

    // Validate assigned_user_id is org member when provided.
    let assigned_user_id = match &request.assigned_user_id {
        None => None,
        Some(uid) => {
            let user_id = UserId::from_string(uid)
                .map_err(|_| AppError::Validation("Invalid assigned_user_id".to_string()))?;
            let org_id = OrganizationId::from_string(&project.organization_id)
                .map_err(|_| AppError::Validation("Invalid assigned_user_id".to_string()))?;
            let is_member = db::organizations::is_member(&state.db, &org_id, &user_id).await?;
            if !is_member {
                return Err(AppError::Validation(
                    "User is not a member of this organization".to_string(),
                ));
            }
            Some(uid.clone())
        }
    };

    // Validate optional group_id (used as node.parent_id) refers to a node in this project.
    let group_parent_id = match &request.group_id {
        None => None,
        Some(pid) => {
            let parent = db::nodes::find_by_id(&state.db, pid)
                .await?
                .ok_or_else(|| AppError::Validation("Invalid group_id".to_string()))?;
            if parent.project_id != project_id {
                return Err(AppError::Validation("Invalid group_id".to_string()));
            }
            Some(pid.clone())
        }
    };

    // Validate that both edge endpoints exist and belong to the project.
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

    // Generate ULID for the new node.
    let node_id = Ulid::new().to_string();

    // Build new node payload.
    let new_node = db::nodes::NewNode {
        id: node_id.clone(),
        project_id: project.id.clone(),
        node_type_id: request.node_type_id.clone(),
        status_id,
        title: request.title.clone(),
        description: request.description.clone(),
        estimated_minutes: None,
        slot_id,
        parent_id: group_parent_id,
        assigned_user_id,
    };

    // Transactionally: insert node, delete old edge, add two new edges.
    let mut tx = state.db.begin().await.map_err(AppError::Database)?;

    db::nodes::insert(&mut *tx, &new_node)
        .await
        .map_err(AppError::Database)?;

    db::node_edges::delete_with_executor(&mut *tx, &request.parent_id, &request.child_id)
        .await
        .map_err(AppError::Database)?;

    let first_edge = db::node_edges::NewNodeEdge {
        parent_id: request.parent_id.clone(),
        child_id: node_id.clone(),
    };
    db::node_edges::insert(&mut *tx, &first_edge)
        .await
        .map_err(AppError::Database)?;

    let second_edge = db::node_edges::NewNodeEdge {
        parent_id: node_id.clone(),
        child_id: request.child_id.clone(),
    };
    db::node_edges::insert(&mut *tx, &second_edge)
        .await
        .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    // Load the node for response (captures created_at/updated_at).
    let node = db::nodes::find_by_id(&state.db, &node_id)
        .await?
        .ok_or_else(|| AppError::Internal)?;

    let response = crate::app::features::graph::create_node::NodeResponse {
        id: node.id,
        project_id: node.project_id,
        node_type_id: node.node_type_id,
        status_id: node.status_id,
        title: node.title,
        description: node.description,
        created_at: node.created_at,
        updated_at: node.updated_at,
        estimated_minutes: node.estimated_minutes,
        slot_id: node.slot_id,
        parent_id: node.parent_id,
        assigned_user_id: node.assigned_user_id,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Insert-between routes.
pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/api/projects/:project_id/edges/insert-between",
        post(insert_between),
    )
}

