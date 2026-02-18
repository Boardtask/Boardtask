use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::app::{
    db,
    error::AppError,
    AppState,
};

/// Response for task statuses.
#[derive(Debug, Serialize)]
pub struct TaskStatusesResponse {
    pub task_statuses: Vec<db::task_statuses::TaskStatus>,
}

/// GET /api/task-statuses — Get all system task statuses.
pub async fn get_task_statuses(
    State(state): State<AppState>,
) -> Result<Json<TaskStatusesResponse>, AppError> {
    let task_statuses = db::task_statuses::get_all_task_statuses(&state.db).await?;
    Ok(Json(TaskStatusesResponse { task_statuses }))
}

/// Task status routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/task-statuses", get(get_task_statuses))
}
