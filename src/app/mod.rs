use axum::Router;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Human-readable application name, used in templates and UI.
/// Change this constant to rename the app across all pages.
pub const APP_NAME: &str = "Boardtask";

/// In-memory cooldown for resend verification (email -> last_sent_at).
pub type ResendCooldown = Arc<std::sync::RwLock<HashMap<String, Instant>>>;

/// Shared state available to all handlers via Axum's state extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub mail: Arc<dyn crate::app::mail::EmailSender>,
    pub config: crate::app::config::Config,
    pub resend_cooldown: ResendCooldown,
}

/// App routes (auth, dashboard). Merged with site routes in main.rs.
pub fn routes(_state: AppState) -> Router<AppState> {
    Router::new()
        .merge(features::auth::routes())
        .merge(features::dashboard::routes())
        .merge(features::account::routes())
        .merge(features::integrations::routes())
        .merge(features::invites::routes())
        .merge(features::organization::routes())
        .merge(features::projects::routes())
        .merge(features::projects::api_routes())
        .merge(features::graph::api::routes())
}

pub mod config;
pub mod domain;
pub mod db;
pub mod single_writer;
pub mod session;
pub mod tenant;
pub mod error;
pub mod features;
pub mod mail;