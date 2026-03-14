use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};

use crate::app::{
    db,
    domain::UserId,
    session::AuthenticatedSession,
    tenant,
    AppState, APP_NAME,
};

/// Integrations page template.
#[derive(Template)]
#[template(path = "integrations.html")]
pub struct IntegrationsTemplate {
    pub app_name: &'static str,
    pub integrations: Vec<db::integrations::Integration>,
    pub current_user_avatar_url: String,
}

/// GET /app/integrations — List allowed integrations with "coming soon" message (org-scoped).
pub async fn show(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if tenant::require_org_member(&state.db, &session.user_id, &session.organization_id)
        .await
        .is_err()
    {
        return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response();
    }

    let integrations = match db::integrations::list_for_org(&state.db, &session.organization_id).await {
        Ok(list) => list,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()).into_response(),
    };

    let user_id = match UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid session".to_string()).into_response(),
    };
    let current_user_avatar_url =
        db::users::profile_image_url_for(&state.db, &user_id).await;

    let template = IntegrationsTemplate {
        app_name: APP_NAME,
        integrations,
        current_user_avatar_url,
    };

    match template.render() {
        Ok(html) => axum::response::Html(html).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error".to_string()).into_response(),
    }
}

/// Integrations routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/app/integrations", get(show))
}
