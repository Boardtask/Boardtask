use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    Form, routing::get, Router,
};
use serde::Deserialize;
use std::time::Duration;
use time::{Duration as TimeDuration, OffsetDateTime};
use validator::Validate;

use crate::app::{
    db,
    domain::{Email, UserId},
    mail::EmailMessage,
    AppState, APP_NAME,
};

/// Resend verification form data from HTTP request.
#[derive(Debug, Deserialize, Validate)]
pub struct ResendVerificationForm {
    #[validate(length(min = 1, max = 254), email)]
    pub email: String,
    /// Optional redirect after verify (from check-email page). Not validated.
    pub next: Option<String>,
}

/// Resend verification page template.
#[derive(Template)]
#[template(path = "resend_verification.html")]
pub struct ResendVerificationTemplate {
    pub app_name: &'static str,
    pub error: String,
    pub email: String,
    /// Safe next URL for hidden form field (empty if not set).
    pub next: String,
}

/// Query parameters for resend verification page.
#[derive(Debug, Deserialize)]
pub struct ResendQuery {
    pub email: Option<String>,
    pub next: Option<String>,
}

/// Safe redirect path: only allow relative paths starting with / to avoid open redirect.
fn safe_redirect_next(next: Option<String>) -> String {
    match next {
        Some(n) if n.starts_with('/') && !n.starts_with("//") => n,
        _ => String::new(),
    }
}

/// GET /resend-verification — Show form (optionally prefilled from ?email=... and ?next=...).
pub async fn show(Query(query): Query<ResendQuery>) -> ResendVerificationTemplate {
    let next = safe_redirect_next(query.next);
    ResendVerificationTemplate {
        app_name: APP_NAME,
        error: String::new(),
        email: query.email.unwrap_or_default(),
        next,
    }
}

/// POST /resend-verification — Process resend, always redirect to check-email (no user enumeration).
pub async fn submit(
    State(state): State<AppState>,
    Form(form): Form<ResendVerificationForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let safe_next = safe_redirect_next(form.next.clone());

    if let Err(_) = form.validate() {
        let template = ResendVerificationTemplate {
            app_name: APP_NAME,
            error: "Invalid email address".to_string(),
            email: form.email.clone(),
            next: safe_next.clone(),
        };
        return Err(Html(template.render().map_err(|_| "Template error".to_string())?));
    }

    let email = match Email::new(form.email.clone()) {
        Ok(e) => e,
        Err(_) => {
            let template = ResendVerificationTemplate {
                app_name: APP_NAME,
                error: "Invalid email address".to_string(),
                email: form.email.clone(),
                next: safe_next.clone(),
            };
            return Err(Html(template.render().map_err(|_| "Template error".to_string())?));
        }
    };

    let email_str = email.as_str().to_string();

    // Rate limit: 1 per 60 seconds per email
    let should_send = {
        let mut cooldown = state.resend_cooldown.write().unwrap();
        let now = std::time::Instant::now();
        let last = cooldown.get(&email_str).copied();
        if let Some(last_sent) = last {
            if last_sent.elapsed() < Duration::from_secs(60) {
                false
            } else {
                cooldown.insert(email_str.clone(), now);
                true
            }
        } else {
            cooldown.insert(email_str.clone(), now);
            true
        }
    };

    let sent = if should_send {
        match db::find_by_email(&state.db, &email).await {
            Ok(Some(user)) if user.email_verified_at.is_none() => {
                let user_id = UserId::from_string(&user.id).unwrap();
                let token = UserId::new().as_str().to_string();
                let expires_at = OffsetDateTime::now_utc() + TimeDuration::hours(72);

                let mut tx = state.db.begin().await.map_err(|_| {
                    let t = ResendVerificationTemplate {
                        app_name: APP_NAME,
                        error: "An error occurred. Please try again.".to_string(),
                        email: form.email.clone(),
                        next: safe_next.clone(),
                    };
                    Html(t.render().unwrap_or_else(|_| "Template error".into()))
                })?;

                db::email_verification::delete_tokens_for_user(&mut *tx, &user_id)
                    .await
                    .map_err(|_| {
                        let t = ResendVerificationTemplate {
                            app_name: APP_NAME,
                            error: "An error occurred. Please try again.".to_string(),
                            email: form.email.clone(),
                            next: safe_next.clone(),
                        };
                        Html(t.render().unwrap_or_else(|_| "Template error".into()))
                    })?;

                db::email_verification::insert_token(&mut *tx, &user_id, &token, expires_at)
                    .await
                    .map_err(|_| {
                        let t = ResendVerificationTemplate {
                            app_name: APP_NAME,
                            error: "An error occurred. Please try again.".to_string(),
                            email: form.email.clone(),
                            next: safe_next.clone(),
                        };
                        Html(t.render().unwrap_or_else(|_| "Template error".into()))
                    })?;

                tx.commit().await.map_err(|_| {
                    let t = ResendVerificationTemplate {
                        app_name: APP_NAME,
                        error: "An error occurred. Please try again.".to_string(),
                        email: form.email.clone(),
                        next: safe_next.clone(),
                    };
                    Html(t.render().unwrap_or_else(|_| "Template error".into()))
                })?;

                let url = if safe_next.is_empty() {
                    format!("{}/verify-email?token={}", state.config.app_url_base(), token)
                } else {
                    format!(
                        "{}/verify-email?token={}&next={}",
                        state.config.app_url_base(),
                        token,
                        urlencoding::encode(&safe_next)
                    )
                };

                let _ = state.mail.send(&EmailMessage::new(
                    email.clone(),
                    "Verify your email".to_string(),
                    format!("Click here to verify your account: {}", url),
                    state.config.mail_from.clone(),
                )).await;

                true
            }
            _ => false,
        }
    } else {
        false
    };

    let redirect_url = if sent {
        if safe_next.is_empty() {
            format!("/check-email?email={}&sent=1", urlencoding::encode(&form.email))
        } else {
            format!(
                "/check-email?email={}&sent=1&next={}",
                urlencoding::encode(&form.email),
                urlencoding::encode(&safe_next)
            )
        }
    } else {
        if safe_next.is_empty() {
            format!("/check-email?email={}", urlencoding::encode(&form.email))
        } else {
            format!(
                "/check-email?email={}&next={}",
                urlencoding::encode(&form.email),
                urlencoding::encode(&safe_next)
            )
        }
    };

    Ok(Redirect::to(&redirect_url))
}

/// Resend verification routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/resend-verification", get(show).post(submit))
}
