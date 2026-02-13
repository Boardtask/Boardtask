use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form, routing::get, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::Deserialize;
use validator::Validate;

use crate::app::{
    domain::{Email, Password},
    error::AppError,
    AppState, APP_NAME,
};
use crate::app::features::auth::service;

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

    // Call service
    match service::signup(&state.db, &email, &password).await {
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