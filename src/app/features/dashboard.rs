use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};

use crate::app::{
    db,
    domain::UserId,
    session::AuthenticatedSession,
    AppState, APP_NAME,
};

/// Dashboard page template.
#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub app_name: &'static str,
    pub display_name: String,
}

/// GET /app — Show dashboard. Requires a valid session; redirects to /login if unauthenticated.
pub async fn show(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user_id = match UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return Redirect::to("/login").into_response(),
    };
    let user = match db::users::find_by_id(&state.db, &user_id).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/login").into_response(),
    };
    let display_name = db::users::display_name(&user);
    let template = DashboardTemplate {
        app_name: APP_NAME,
        display_name,
    };
    Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response()
}

/// Dashboard routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/app", get(show))
}