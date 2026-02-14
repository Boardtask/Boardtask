pub mod app;
pub mod site;

use axum::Router;

/// Build the full application router. Used by main and by integration tests.
pub fn create_router(state: app::AppState) -> Router {
    Router::new()
        .merge(site::home::routes())
        .merge(app::routes(state.clone()))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}
