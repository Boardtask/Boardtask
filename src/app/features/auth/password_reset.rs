use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    Form, routing::get, Router,
};
use serde::Deserialize;
use time::{Duration, OffsetDateTime};
use validator::Validate;
use rand_core::RngCore;

use crate::app::{
    db,
    domain::{Email, Password, HashedPassword, UserId},
    mail::EmailMessage,
    AppState, APP_NAME,
};

/// Forgot password form data from HTTP request.
#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordForm {
    #[validate(length(min = 1, max = 254), email)]
    pub email: String,
}

/// Forgot password page template.
#[derive(Template)]
#[template(path = "forgot_password.html")]
pub struct ForgotPasswordTemplate {
    pub app_name: &'static str,
    pub error: String,
    pub email: String,
    pub success: bool,
}

/// Reset password form data from HTTP request.
#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordForm {
    pub token: String,

    #[validate(length(min = 8, max = 128))]
    pub password: String,

    #[validate(must_match(other = "password"))]
    pub confirm_password: String,
}

/// Reset password page template.
#[derive(Template)]
#[template(path = "reset_password.html")]
pub struct ResetPasswordTemplate {
    pub app_name: &'static str,
    pub error: String,
    pub token: String,
}

/// Query parameters for forgot password page (error handling).
#[derive(Debug, Deserialize)]
pub struct ForgotQuery {
    pub error: Option<String>,
}

/// Query parameters for reset password page.
#[derive(Debug, Deserialize)]
pub struct ResetQuery {
    pub token: Option<String>,
}

/// Generate a cryptographically secure random token.
fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// GET /forgot-password — Show forgot password form.
pub async fn show_forgot(
    Query(query): Query<ForgotQuery>,
) -> ForgotPasswordTemplate {
    ForgotPasswordTemplate {
        app_name: APP_NAME,
        error: query.error.unwrap_or_default(),
        email: String::new(),
        success: false,
    }
}

/// POST /forgot-password — Process forgot password form.
pub async fn submit_forgot(
    State(state): State<AppState>,
    Form(form): Form<ForgotPasswordForm>,
) -> Result<impl IntoResponse, Html<String>> {
    // Validate form structure
    if let Err(_) = form.validate() {
        let template = ForgotPasswordTemplate {
            app_name: APP_NAME,
            error: "Invalid form data".to_string(),
            email: form.email.clone(),
            success: false,
        };
        return Err(Html(template.render().map_err(|_| "Template error".to_string())?));
    }

    // Parse into domain types
    let email = match Email::new(form.email.clone()) {
        Ok(email) => email,
        Err(_) => {
            let template = ForgotPasswordTemplate {
                app_name: APP_NAME,
                error: "Invalid email address".to_string(),
                email: form.email.clone(),
                success: false,
            };
            return Err(Html(template.render().map_err(|_| "Template error".to_string())?));
        }
    };

    // Check if user exists and send email
    match db::find_by_email(&state.db, &email).await {
        Ok(Some(user)) => {
            // User exists, generate and send reset token
            let token = generate_token();
            let expires_at = OffsetDateTime::now_utc() + Duration::hours(1);

            if let Ok(()) = db::password_reset::insert_token(&state.db, &UserId::from_string(&user.id).unwrap(), &token, expires_at).await {
                // Send email only when token was stored (link would be useless otherwise)
                let reset_link = format!("{}/reset-password?token={}", state.config.app_url_base(), token);
                let message = EmailMessage::new(
                    email,
                    "Reset your Boardtask password".to_string(),
                    format!("Click this link to reset your password: {}", reset_link),
                    state.config.mail_from.clone(),
                );
                let _ = state.mail.send(&message).await; // Ignore send errors
            }
        }
        _ => {
            // User doesn't exist or database error - show same success message
        }
    }

    // Always show success message (no user enumeration)
    let template = ForgotPasswordTemplate {
        app_name: APP_NAME,
        error: String::new(),
        email: String::new(),
        success: true,
    };

    Ok(Html(template.render().map_err(|_| "Template error".to_string())?))
}

/// GET /reset-password — Show reset password form.
pub async fn show_reset(
    State(state): State<AppState>,
    Query(query): Query<ResetQuery>,
) -> impl IntoResponse {
    let token = match query.token {
        Some(t) if !t.is_empty() => t,
        _ => return Redirect::to("/forgot-password?error=invalid").into_response(),
    };

    // Validate token
    match db::password_reset::find_valid_token(&state.db, &token).await {
        Ok(Some(_)) => {
            // Token is valid, show form
            let template = ResetPasswordTemplate {
                app_name: APP_NAME,
                error: String::new(),
                token,
            };
            Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response()
        }
        _ => {
            // Token invalid or expired
            Redirect::to("/forgot-password?error=invalid").into_response()
        }
    }
}

/// POST /reset-password — Process reset password form.
pub async fn submit_reset(
    State(state): State<AppState>,
    Form(form): Form<ResetPasswordForm>,
) -> impl IntoResponse {
    // Validate form structure
    if let Err(_) = form.validate() {
        let template = ResetPasswordTemplate {
            app_name: APP_NAME,
            error: "Invalid form data".to_string(),
            token: form.token.clone(),
        };
        return Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
    }

    // Validate token
    let user_id = match db::password_reset::find_valid_token(&state.db, &form.token).await {
        Ok(Some(id)) => id,
        _ => {
            let template = ResetPasswordTemplate {
                app_name: APP_NAME,
                error: "Invalid or expired reset token".to_string(),
                token: form.token.clone(),
            };
            return Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
        }
    };

    // Parse password
    let password = match Password::new(form.password) {
        Ok(password) => password,
        Err(e) => {
            let template = ResetPasswordTemplate {
                app_name: APP_NAME,
                error: e.message.unwrap_or_else(|| "Invalid password".into()).to_string(),
                token: form.token.clone(),
            };
            return Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
        }
    };

    // Hash password and update user in transaction
    let password_hash = match HashedPassword::from_password(&password) {
        Ok(hash) => hash,
        Err(_) => {
            let template = ResetPasswordTemplate {
                app_name: APP_NAME,
                error: "Password hashing failed".to_string(),
                token: form.token.clone(),
            };
            return Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
        }
    };

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            let template = ResetPasswordTemplate {
                app_name: APP_NAME,
                error: "Database error".to_string(),
                token: form.token.clone(),
            };
            return Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
        }
    };

    if let Err(_) = db::update_password(&mut *tx, &user_id, &password_hash).await {
        let _ = tx.rollback().await;
        let template = ResetPasswordTemplate {
            app_name: APP_NAME,
            error: "Failed to update password".to_string(),
            token: form.token.clone(),
        };
        return Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
    }

    if let Err(_) = db::password_reset::delete_token(&mut *tx, &form.token).await {
        let _ = tx.rollback().await;
        let template = ResetPasswordTemplate {
            app_name: APP_NAME,
            error: "Failed to invalidate reset token".to_string(),
            token: form.token.clone(),
        };
        return Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
    }

    if let Err(_) = tx.commit().await {
        let template = ResetPasswordTemplate {
            app_name: APP_NAME,
            error: "Database transaction failed".to_string(),
            token: form.token.clone(),
        };
        return Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
    }

    // Success - redirect to login
    Redirect::to("/login?reset=ok").into_response()
}

/// Password reset routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/forgot-password", get(show_forgot).post(submit_forgot))
        .route("/reset-password", get(show_reset).post(submit_reset))
}