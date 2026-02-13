use axum::Router;
use sqlx::SqlitePool;

/// Human-readable application name, used in templates and UI.
/// Change this constant to rename the app across all pages.
pub const APP_NAME: &str = "Boardtask";

/// Shared state available to all handlers via Axum's state extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
}

/// App routes (auth, dashboard). Merged with site routes in main.rs.
pub fn routes(_state: AppState) -> Router<AppState> {
    Router::new()
        .merge(features::auth::routes())
        .route("/app", axum::routing::get(dashboard))
}

/// Placeholder dashboard handler.
async fn dashboard() -> axum::response::Html<&'static str> {
    axum::response::Html(r#"
        <!DOCTYPE html>
        <html>
        <head><title>Dashboard</title></head>
        <body>
            <h1>Welcome to your dashboard!</h1>
            <p>This is a placeholder dashboard. Authentication successful!</p>
            <a href="/">Back to home</a>
        </body>
        </html>
    "#)
}

pub mod domain;
pub mod db;
pub mod error;
pub mod features;