use axum::Router;
use sqlx::SqlitePool;
use std::sync::Arc;

/// Human-readable application name, used in templates and UI.
/// Change this constant to rename the app across all pages.
pub const APP_NAME: &str = "Boardtask";

/// Shared state available to all handlers via Axum's state extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub mail: Arc<dyn crate::app::mail::EmailSender>,
    pub config: crate::app::config::Config,
}

/// App routes (auth, dashboard). Merged with site routes in main.rs.
pub fn routes(_state: AppState) -> Router<AppState> {
    Router::new()
        .merge(features::auth::routes())
        .merge(features::dashboard::routes())
}

pub mod config;
pub mod domain;
pub mod db;
pub mod session;
pub mod error;
pub mod features;
pub mod mail;