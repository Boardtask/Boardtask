pub mod app;
pub mod site;

use axum::Router;
use sqlx::SqlitePool;

/// Build the full application router. Used by main and by integration tests.
pub fn create_router(pool: SqlitePool) -> Router {
    let state = app::AppState { db: pool };
    Router::new()
        .merge(site::home::routes())
        .merge(app::routes(state.clone()))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}
