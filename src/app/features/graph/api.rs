use axum::Router;

use crate::app::AppState;

/// Graph API routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(crate::app::features::graph::nodes::routes())
        .merge(crate::app::features::graph::edges::routes())
}