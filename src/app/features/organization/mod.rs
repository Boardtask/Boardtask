mod settings;

use axum::Router;

use crate::app::AppState;

/// Organization settings and invite routes.
pub fn routes() -> Router<AppState> {
    Router::new().merge(settings::routes())
}
