use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use crate::app::{
    db,
    error::AppError,
    AppState, APP_NAME,
};

#[derive(Debug, Deserialize)]
pub struct VerifyQuery {
    pub token: String,
}

#[derive(Template)]
#[template(path = "verify_email.html")]
pub struct VerifyEmailTemplate {
    pub app_name: &'static str,
    pub error: String,
}

#[derive(Template)]
#[template(path = "check_email.html")]
pub struct CheckEmailTemplate {
    pub app_name: &'static str,
    pub email: String,
}

/// Error type for verify-email route. Renders HTML instead of JSON.
enum VerifyError {
    BadRequest(String),
    Database,
}

impl From<AppError> for VerifyError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::Auth(msg) => VerifyError::BadRequest(msg),
            _ => VerifyError::Database,
        }
    }
}

impl IntoResponse for VerifyError {
    fn into_response(self) -> Response {
        let msg = match self {
            VerifyError::BadRequest(m) => m,
            VerifyError::Database => "An error occurred. Please try again.".to_string(),
        };
        let t = VerifyEmailTemplate { app_name: APP_NAME, error: msg };
        Html(t.render().unwrap_or_else(|_| "An error occurred.".into())).into_response()
    }
}

/// Validate token, mark user verified, create session. Returns session_id on success.
async fn verify_user(db: &sqlx::SqlitePool, token: &str) -> Result<String, AppError> {
    let user_id = db::email_verification::find_valid_token(db, token)
        .await?
        .ok_or_else(|| AppError::Auth("Invalid or expired verification link.".to_string()))?;

    let mut tx = db.begin().await?;

    db::users::mark_verified(&mut *tx, &user_id).await?;
    db::email_verification::delete_token(&mut *tx, token).await?;

    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);
    let session_id = db::sessions::create(&mut *tx, &user_id, expires_at).await?;

    tx.commit().await?;

    Ok(session_id)
}

/// GET /check-email — Shown after signup. Display email for user to check.
pub async fn check_email_page(Query(params): Query<std::collections::HashMap<String, String>>) -> CheckEmailTemplate {
    let email = params.get("email").cloned().unwrap_or_else(|| "your email".to_string());
    CheckEmailTemplate {
        app_name: APP_NAME,
        email,
    }
}

/// GET /verify-email?token=... — Validate token, mark user verified, create session, redirect to /app.
async fn verify(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<VerifyQuery>,
) -> Result<impl IntoResponse, VerifyError> {
    if query.token.is_empty() {
        return Err(VerifyError::BadRequest("Missing verification token.".to_string()));
    }

    let session_id = verify_user(&state.db, &query.token)
        .await
        .map_err(VerifyError::from)?;

    Ok((jar.add(crate::app::session::session_cookie(session_id)), Redirect::to("/app")))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/check-email", get(check_email_page))
        .route("/verify-email", get(verify))
}