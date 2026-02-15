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

/// Response for node types.
#[derive(Debug, Serialize)]
pub struct NodeTypesResponse {
    pub node_types: Vec<db::node_types::NodeType>,
}

/// GET /api/node-types — Get all system node types.
pub async fn get_node_types(
    State(state): State<AppState>,
) -> Result<Json<NodeTypesResponse>, AppError> {
    let node_types = db::node_types::get_all_systems(&state.db).await?;
    Ok(Json(NodeTypesResponse { node_types }))
}

/// Node type routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/node-types", get(get_node_types))
}
