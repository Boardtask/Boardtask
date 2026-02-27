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
    /// Optional redirect after verify (e.g. /accept-invite/confirm?token=...).
    pub next: Option<String>,
}

/// Safe redirect path: only allow relative paths starting with / to avoid open redirect.
fn safe_redirect_next(next: Option<String>) -> String {
    match next {
        Some(n) if n.starts_with('/') && !n.starts_with("//") => n,
        _ => "/app".to_string(),
    }
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
    pub sent: bool,
    /// Safe next URL for resend form (empty if not set).
    pub next: String,
    /// Login link: either "/login" or "/login?next=..." (next already URL-encoded).
    pub login_url: String,
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

    let user = db::users::find_by_id(&mut *tx, &user_id)
        .await?
        .ok_or_else(|| AppError::Internal)?;

    let org_id = crate::app::domain::OrganizationId::from_string(&user.organization_id)
        .map_err(|_| AppError::Internal)?;

    db::users::mark_verified(&mut *tx, &user_id).await?;
    db::email_verification::delete_token(&mut *tx, token).await?;

    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);
    let session_id = db::sessions::create(&mut *tx, &user_id, &org_id, expires_at).await?;

    tx.commit().await?;

    Ok(session_id)
}

/// Safe redirect path for check-email: only allow relative paths starting with /.
fn safe_next(next: Option<&String>) -> String {
    match next {
        Some(n) if n.starts_with('/') && !n.starts_with("//") => n.clone(),
        _ => String::new(),
    }
}

/// GET /check-email — Shown after signup. Display email for user to check.
pub async fn check_email_page(Query(params): Query<std::collections::HashMap<String, String>>) -> CheckEmailTemplate {
    let email = params.get("email").cloned().unwrap_or_else(|| "your email".to_string());
    let sent = params.get("sent").map(|s| s == "1").unwrap_or(false);
    let next = safe_next(params.get("next"));
    let login_url = if next.is_empty() {
        "/login".to_string()
    } else {
        format!("/login?next={}", urlencoding::encode(&next))
    };
    CheckEmailTemplate {
        app_name: APP_NAME,
        email,
        sent,
        next,
        login_url,
    }
}

/// GET /verify-email?token=... — Validate token, mark user verified, create session, redirect to next or /app.
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

    let redirect_to = safe_redirect_next(query.next);
    Ok((jar.add(crate::app::session::session_cookie(session_id)), Redirect::to(&redirect_to)))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/check-email", get(check_email_page))
        .route("/verify-email", get(verify))
}