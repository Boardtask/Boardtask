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
    AppState, APP_NAME,
};

/// Login form data from HTTP request.
#[derive(Debug, Deserialize, Validate)]
pub struct LoginForm {
    #[validate(length(min = 1, max = 254), email)]
    pub email: String,

    #[validate(length(min = 1))]
    pub password: String,
}

/// Login page template.
#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub app_name: &'static str,
    pub error: String,
}

/// Authenticate a user. Returns the session ID on success.
async fn authenticate(
    pool: &sqlx::SqlitePool,
    email: &Email,
    password: &Password,
) -> Result<String, AppError> {
    // Find user by email
    let user = db::find_by_email(pool, email)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::Auth("Invalid email or password".to_string()))?;

    // Verify password
    let stored_hash = HashedPassword::from_string(user.password_hash);
    stored_hash.verify(password)
        .map_err(|_| AppError::Auth("Invalid email or password".to_string()))?;

    // Check if email is verified
    if user.email_verified_at.is_none() {
        return Err(AppError::Auth("Please verify your email before signing in. Check your inbox for the verification link.".to_string()));
    }

    // Parse user ID
    let user_id = UserId::from_string(&user.id)
        .map_err(|_| AppError::Internal)?;

    // Create session (30 days)
    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);
    let session_id = db::create(pool, &user_id, expires_at)
        .await
        .map_err(AppError::Database)?;

    Ok(session_id)
}

/// GET /login — Show login form.
pub async fn show() -> LoginTemplate {
    LoginTemplate {
        app_name: APP_NAME,
        error: String::new(),
    }
}

/// POST /login — Process login form.
pub async fn submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, Html<String>> {
    // Validate form structure
    if let Err(_) = form.validate() {
        let template = LoginTemplate {
            app_name: APP_NAME,
            error: "Invalid form data".to_string(),
        };
        return Err(Html(template.render().map_err(|_| "Template error".to_string())?));
    }

    // Parse into domain types
    let email = match Email::new(form.email) {
        Ok(email) => email,
        Err(_) => {
            let template = LoginTemplate {
                app_name: APP_NAME,
                error: "Invalid email or password".to_string(),
            };
            return Err(Html(template.render().map_err(|_| "Template error".to_string())?));
        }
    };

    // Use for_verification—no strength check. We only verify against the stored hash.
    // Strength rules apply at signup, not login (legacy accounts may have weaker passwords).
    let password = Password::for_verification(form.password);

    // Authenticate
    match authenticate(&state.db, &email, &password).await {
        Ok(session_id) => {
            Ok((jar.add(crate::app::session::session_cookie(session_id)), Redirect::to("/app")))
        }
        Err(AppError::Auth(msg)) => {
            let template = LoginTemplate {
                app_name: APP_NAME,
                error: msg,
            };
            Err(Html(template.render().map_err(|_| "Template error".to_string())?))
        }
        Err(_) => {
            let template = LoginTemplate {
                app_name: APP_NAME,
                error: "Internal server error".to_string(),
            };
            Err(Html(template.render().map_err(|_| "Template error".to_string())?))
        }
    }
}

/// Login routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", get(show).post(submit))
}