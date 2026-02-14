use axum::Router;

use crate::app::AppState;

/// Graph API routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(crate::app::features::graph::create_node::routes())
        .merge(crate::app::features::graph::update_node::routes())
        .merge(crate::app::features::graph::delete_node::routes())
        .merge(crate::app::features::graph::create_edge::routes())
        .merge(crate::app::features::graph::delete_edge::routes())
}