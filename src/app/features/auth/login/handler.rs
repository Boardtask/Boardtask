use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};

use crate::app::{
    domain::{Email, Password},
    error::AppError,
    AppState, APP_NAME,
};
use validator::Validate;
use crate::app::features::auth::service;
use super::form::LoginForm;

/// Login page template.
#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub app_name: &'static str,
    pub error: String,
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

    // Call service
    match service::login(&state.db, &email, &password).await {
        Ok(session_id) => {
            // Set session cookie
            let cookie = Cookie::build(("session_id", session_id))
                .http_only(true)
                .same_site(axum_extra::extract::cookie::SameSite::Lax)
                .path("/")
                .build();

            let jar = jar.add(cookie);

            // Redirect to dashboard
            Ok((jar, Redirect::to("/app")))
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