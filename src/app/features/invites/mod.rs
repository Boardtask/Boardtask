mod accept;

use axum::Router;

use crate::app::AppState;

/// Public invite acceptance routes (no auth required for GET).
pub fn routes() -> Router<AppState> {
    Router::new().merge(accept::routes())
}
