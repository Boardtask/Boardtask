use askama::Template;
use axum::{routing::get, Router};

use crate::app::{AppState, APP_NAME};

/// Dashboard page template.
#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub app_name: &'static str,
}

/// GET /app — Show dashboard.
pub async fn show() -> DashboardTemplate {
    DashboardTemplate {
        app_name: APP_NAME,
    }
}

/// Dashboard routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/app", get(show))
}