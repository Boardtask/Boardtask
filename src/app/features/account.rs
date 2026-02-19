use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    Form,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use validator::Validate;

use crate::app::{
    db,
    domain::{Password, HashedPassword, UserId},
    session::AuthenticatedSession,
    AppState, APP_NAME,
};

/// Query parameters for account page (error/success feedback).
#[derive(Debug, Deserialize)]
pub struct AccountQuery {
    pub error: Option<String>,
    pub success: Option<String>,
}

/// Account page template.
#[derive(Template)]
#[template(path = "account.html")]
pub struct AccountTemplate {
    pub app_name: &'static str,
    pub email: String,
    pub email_verified: bool,
    pub error: String,
    pub success: String,
}

/// Change password form data.
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordForm {
    #[validate(length(min = 1))]
    pub current_password: String,

    #[validate(length(min = 8, max = 128))]
    pub new_password: String,

    #[validate(must_match(other = "new_password"))]
    pub confirm_password: String,
}

fn error_redirect(msg: &str) -> Redirect {
    let encoded = urlencoding::encode(msg);
    Redirect::to(&format!("/app/account?error={}", encoded))
}

/// GET /app/account — Show account info and change-password form.
pub async fn show_account(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Query(query): Query<AccountQuery>,
) -> impl IntoResponse {
    let user_id = match UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let user = match db::users::find_by_id(&state.db, &user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let template = AccountTemplate {
        app_name: APP_NAME,
        email: user.email,
        email_verified: user.email_verified_at.is_some(),
        error: query.error.unwrap_or_default(),
        success: query.success.unwrap_or_default(),
    };

    Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response()
}

/// POST /app/account/change-password — Update password after verifying current one.
/// Validation first (no DB), then load user and verify current password, then write.
pub async fn change_password(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Form(form): Form<ChangePasswordForm>,
) -> impl IntoResponse {
    // 1. Validation first — no database until input is valid
    if form.validate().is_err() {
        return error_redirect("New password and confirmation must match and be 8–128 characters.")
            .into_response();
    }

    let new_password = match Password::new(form.new_password) {
        Ok(p) => p,
        Err(e) => {
            let msg = e
                .message
                .map(|c| c.into_owned())
                .unwrap_or_else(|| "Invalid new password.".to_string());
            return error_redirect(&msg).into_response();
        }
    };

    let password_hash = match HashedPassword::from_password(&new_password) {
        Ok(h) => h,
        Err(_) => return error_redirect("Password hashing failed.").into_response(),
    };

    // 2. Then load user and verify current password
    let user_id = match UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let user = match db::users::find_by_id(&state.db, &user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(_) => return error_redirect("Database error.").into_response(),
    };

    let current = Password::for_verification(form.current_password);
    let stored_hash = HashedPassword::from_string(user.password_hash);
    if stored_hash.verify(&current).is_err() {
        return error_redirect("Current password is wrong.").into_response();
    }

    // 3. Then write
    if db::users::update_password(&state.db, &user_id, &password_hash)
        .await
        .is_err()
    {
        return error_redirect("Failed to update password.").into_response();
    }

    Redirect::to("/app/account?success=password_changed").into_response()
}

/// Account routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/app/account", get(show_account))
        .route("/app/account/change-password", post(change_password))
}
