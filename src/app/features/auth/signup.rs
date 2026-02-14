use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form, routing::get, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};
use validator::Validate;

use crate::app::{
    db,
    domain::{Email, Password, HashedPassword, UserId},
    error::AppError,
    mail::EmailMessage,
    AppState, APP_NAME,
};

/// Signup form data from HTTP request.
#[derive(Debug, Deserialize, Validate)]
pub struct SignupForm {
    #[validate(length(min = 1, max = 254), email)]
    pub email: String,

    #[validate(length(min = 8, max = 128))]
    pub password: String,

    #[validate(must_match(other = "password"))]
    pub confirm_password: String,
}

/// Signup page template.
#[derive(Template)]
#[template(path = "signup.html")]
pub struct SignupTemplate {
    pub app_name: &'static str,
    pub error: String,
    pub email: String,
}

/// Create a new user account and verification token. Returns (user_id, token) on success.
async fn create_account(
    pool: &sqlx::SqlitePool,
    email: &Email,
    password: &Password,
) -> Result<(UserId, String), AppError> {
    // Check if email already exists
    if let Some(_) = db::find_by_email(pool, email).await.map_err(AppError::Database)? {
        return Err(AppError::Auth("Unable to create account. If you already have an account, please log in.".to_string()));
    }

    // Hash the password
    let password_hash = HashedPassword::from_password(password)
        .map_err(|_| AppError::Internal)?;

    // Generate user ID
    let user_id = UserId::new();

    // Create new user
    let new_user = db::NewUser {
        id: user_id.clone(),
        email: email.clone(),
        password_hash,
    };

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    db::insert(&mut *tx, &new_user).await.map_err(AppError::Database)?;

    // Generate verification token
    let token = UserId::new().as_str();
    let expires_at = OffsetDateTime::now_utc() + Duration::hours(24);
    db::email_verification::insert_token(&mut *tx, &new_user.id, &token, expires_at)
        .await.map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    Ok((user_id, token))
}

/// GET /signup — Show signup form.
pub async fn show() -> SignupTemplate {
    SignupTemplate {
        app_name: APP_NAME,
        error: String::new(),
        email: String::new(),
    }
}

/// POST /signup — Process signup form.
pub async fn submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<SignupForm>,
) -> Result<impl IntoResponse, Html<String>> {
    // Validate form structure
    if let Err(_) = form.validate() {
        let template = SignupTemplate {
            app_name: APP_NAME,
            error: "Invalid form data".to_string(),
            email: form.email.clone(),
        };
        return Err(Html(template.render().map_err(|_| "Template error".to_string())?));
    }

    // Parse into domain types
    let email = match Email::new(form.email.clone()) {
        Ok(email) => email,
        Err(_) => {
            let template = SignupTemplate {
                app_name: APP_NAME,
                error: "Invalid email address".to_string(),
                email: form.email.clone(),
            };
            return Err(Html(template.render().map_err(|_| "Template error".to_string())?));
        }
    };

    let password = match Password::new(form.password) {
        Ok(password) => password,
        Err(e) => {
            let template = SignupTemplate {
                app_name: APP_NAME,
                error: e.message.unwrap_or_else(|| "Invalid password".into()).to_string(),
                email: form.email.clone(),
            };
            return Err(Html(template.render().map_err(|_| "Template error".to_string())?));
        }
    };

    // Create account
    match create_account(&state.db, &email, &password).await {
        Ok((_user_id, token)) => {
            let base = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".into());
            let url = format!("{}/verify-email?token={}", base.trim_end_matches('/'), token);
            match state.mail.send(&EmailMessage::new(
                email.clone(),
                "Verify your email".to_string(),
                format!("Click here to verify your account: {}", url),
            )).await {
                Ok(()) => Ok((jar, Redirect::to(&format!("/check-email?email={}", email.as_str())))),
                Err(_) => {
                    let template = SignupTemplate {
                        app_name: APP_NAME,
                        error: "Failed to send verification email.".to_string(),
                        email: form.email.clone(),
                    };
                    Err(Html(template.render().map_err(|_| "Template error".to_string())?))
                }
            }
        }
        Err(AppError::Auth(msg)) => {
            let template = SignupTemplate {
                app_name: APP_NAME,
                error: msg,
                email: form.email.clone(),
            };
            Err(Html(template.render().map_err(|_| "Template error".to_string())?))
        }
        Err(_) => {
            let template = SignupTemplate {
                app_name: APP_NAME,
                error: "Internal server error".to_string(),
                email: form.email.clone(),
            };
            Err(Html(template.render().map_err(|_| "Template error".to_string())?))
        }
    }
}

/// Signup routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/signup", get(show).post(submit))
}