use askama::Template;
use axum::{
    response::IntoResponse,
    routing::get,
    Router,
};

use crate::app::{session::AuthenticatedSession, AppState, APP_NAME};

/// Dashboard page template.
#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub app_name: &'static str,
}

/// GET /app — Show dashboard. Requires a valid session; redirects to /login if unauthenticated.
pub async fn show(AuthenticatedSession(_session): AuthenticatedSession) -> impl IntoResponse {
    DashboardTemplate {
        app_name: APP_NAME,
    }
    .into_response()
}

/// Dashboard routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/app", get(show))
}