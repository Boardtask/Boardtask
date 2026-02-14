use askama::Template;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::CookieJar;

use crate::app::{db, AppState, APP_NAME};

/// Dashboard page template.
#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub app_name: &'static str,
}

/// GET /app — Show dashboard. Requires a valid session; redirects to /login if unauthenticated.
pub async fn show(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let session_id = match jar.get("session_id") {
        Some(c) => c.value().to_string(),
        None => return Redirect::to("/login").into_response(),
    };

    let session = match db::sessions::find_valid(&state.db, &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) | Err(_) => return Redirect::to("/login").into_response(),
    };

    let _ = session; // suppress unused warning; session proves authentication
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