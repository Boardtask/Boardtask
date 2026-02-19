use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
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

/// Path parameters for slot endpoints with ID.
#[derive(Debug, serde::Deserialize)]
pub struct SlotPathParams {
    pub project_id: String,
    pub id: String,
}

/// Response for listing slots.
#[derive(Debug, Serialize)]
pub struct SlotsResponse {
    pub slots: Vec<db::project_slots::ProjectSlot>,
}

/// Request body for creating a slot.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSlotRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub sort_order: Option<i64>,
}

/// Request body for updating a slot.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSlotRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub sort_order: Option<i64>,
}

/// GET /api/projects/:project_id/slots — List slots for a project.
pub async fn list_slots(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<SlotsResponse>, AppError> {
    super::helpers::ensure_project_owned(&state.db, &project_id, &session.user_id, &session.organization_id).await?;
    let slots = db::project_slots::find_by_project(&state.db, &project_id).await?;
    Ok(Json(SlotsResponse { slots }))
}

/// POST /api/projects/:project_id/slots — Create a slot.
pub async fn create_slot(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(request): Json<CreateSlotRequest>,
) -> Result<(StatusCode, Json<db::project_slots::ProjectSlot>), AppError> {
    request
        .validate()
        .map_err(|_| AppError::Validation("Invalid input".to_string()))?;

    super::helpers::ensure_project_owned(&state.db, &project_id, &session.user_id, &session.organization_id).await?;

    let existing = db::project_slots::find_by_project(&state.db, &project_id).await?;
    if existing.iter().any(|s| s.name == request.name) {
        return Err(AppError::Validation("Duplicate slot name".to_string()));
    }

    let sort_order = request.sort_order.unwrap_or(0);
    let slot = db::project_slots::NewProjectSlot {
        id: Ulid::new().to_string(),
        project_id: project_id.clone(),
        name: request.name,
        sort_order,
    };

    db::project_slots::insert(&state.db, &slot).await?;

    let created = db::project_slots::find_by_id(&state.db, &slot.id)
        .await?
        .ok_or_else(|| AppError::Internal)?;

    Ok((StatusCode::CREATED, Json(created)))
}

/// PATCH /api/projects/:project_id/slots/:id — Update a slot.
pub async fn update_slot(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(params): Path<SlotPathParams>,
    Json(request): Json<UpdateSlotRequest>,
) -> Result<Json<db::project_slots::ProjectSlot>, AppError> {
    request
        .validate()
        .map_err(|_| AppError::Validation("Invalid input".to_string()))?;

    super::helpers::ensure_project_owned(&state.db, &params.project_id, &session.user_id, &session.organization_id).await?;

    let slot = db::project_slots::find_by_id(&state.db, &params.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Slot not found".to_string()))?;

    if slot.project_id != params.project_id {
        return Err(AppError::NotFound("Slot not found".to_string()));
    }

    let name = request.name.as_deref().unwrap_or(&slot.name);
    let sort_order = request.sort_order.unwrap_or(slot.sort_order);

    db::project_slots::update(&state.db, &params.id, name, sort_order).await?;

    let updated = db::project_slots::find_by_id(&state.db, &params.id)
        .await?
        .ok_or_else(|| AppError::Internal)?;

    Ok(Json(updated))
}

/// DELETE /api/projects/:project_id/slots/:id — Delete a slot.
pub async fn delete_slot(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(params): Path<SlotPathParams>,
) -> Result<StatusCode, AppError> {
    super::helpers::ensure_project_owned(&state.db, &params.project_id, &session.user_id, &session.organization_id).await?;

    let slot = db::project_slots::find_by_id(&state.db, &params.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Slot not found".to_string()))?;

    if slot.project_id != params.project_id {
        return Err(AppError::NotFound("Slot not found".to_string()));
    }

    db::project_slots::delete(&state.db, &params.id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Slot routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/projects/:project_id/slots", get(list_slots).post(create_slot))
        .route("/api/projects/:project_id/slots/:id", patch(update_slot).delete(delete_slot))
}
