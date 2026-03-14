use askama::Template;
use axum::{
    extract::Request,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json},
};
use serde_json::json;

/// 404 page template ("Lost in Transit").
#[derive(Template)]
#[template(path = "not_found.html")]
pub struct NotFoundTemplate {
    pub app_name: &'static str,
    pub current_user_avatar_url: String,
}

/// Fallback handler for unmatched routes. Returns HTML 404 page for browser
/// requests, or JSON for API requests.
pub async fn handler(req: Request) -> impl IntoResponse {
    let prefers_json = req
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("application/json"))
        .unwrap_or(false);
    let is_api_path = req.uri().path().starts_with("/api/");

    if prefers_json || is_api_path {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Not found" })),
        )
            .into_response()
    } else {
        let template = NotFoundTemplate {
            app_name: crate::app::APP_NAME,
            current_user_avatar_url: String::new(),
        };
        let html = template
            .render()
            .unwrap_or_else(|_| "Page not found".to_string());
        (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Html(html),
        )
            .into_response()
    }
}
