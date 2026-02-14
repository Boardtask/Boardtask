use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::app::{
    db,
    error::AppError,
    session::ApiAuthenticatedSession,
    AppState,
};

/// Response for the full graph (nodes + edges).
#[derive(Debug, Serialize)]
pub struct GraphResponse {
    pub nodes: Vec<db::nodes::Node>,
    pub edges: Vec<db::node_edges::NodeEdge>,
}

/// GET /api/projects/:project_id/graph — Get full project graph.
pub async fn get_graph(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<GraphResponse>, AppError> {
    // Ensure user owns the project
    super::helpers::ensure_project_owned(&state.db, &project_id, &session.user_id).await?;

    // Fetch nodes and edges
    let nodes = db::nodes::find_by_project(&state.db, &project_id).await?;
    let edges = db::node_edges::find_by_project(&state.db, &project_id).await?;

    Ok(Json(GraphResponse { nodes, edges }))
}

/// Graph routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/projects/:project_id/graph", get(get_graph))
}