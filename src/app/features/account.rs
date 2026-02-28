use std::sync::LazyLock;

use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    Form,
    routing::{get, post},
    Router,
};
use regex::Regex;
use serde::Deserialize;
use validator::Validate;

use crate::app::{
    db,
    domain::{HashedPassword, Password, ProfileImageUrl, UserId},
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
    pub first_name_value: String,
    pub last_name_value: String,
    pub profile_image_url_value: String,
    pub profile_image_url_display: String,
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

/// Update name form data.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateNameForm {
    #[validate(length(min = 1, max = 100))]
    pub first_name: String,

    #[validate(length(min = 1, max = 100))]
    pub last_name: String,
}

/// Regex: empty/whitespace or HTTPS URL with image extension (.jpg, .jpeg, .png, .gif, .webp).
static RE_PROFILE_IMAGE_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*$|^\s*https://[^?#]*\.(jpg|jpeg|png|gif|webp)(\?.*)?(#.*)?$").unwrap()
});

/// Update profile image URL form. Empty string means "clear".
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileImageForm {
    #[validate(
        length(max = 2048),
        regex(path = *RE_PROFILE_IMAGE_URL, message = "URL must use HTTPS and point to an image."),
    )]
    pub profile_image_url: Option<String>,
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

    let first_name_value = user.first_name.clone();
    let last_name_value = user.last_name.clone();

    let template = AccountTemplate {
        app_name: APP_NAME,
        email: user.email,
        email_verified: user.email_verified_at.is_some(),
        first_name_value,
        last_name_value,
        profile_image_url_value: user.profile_image_url.as_deref().unwrap_or("").to_string(),
        profile_image_url_display: user.profile_image_url.clone().unwrap_or_default(),
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

/// POST /app/account/update-name — Update first and last name. Validate first, then write.
pub async fn update_name(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Form(form): Form<UpdateNameForm>,
) -> impl IntoResponse {
    if form.validate().is_err() {
        return error_redirect("First and last name are required (1–100 characters each).")
            .into_response();
    }

    let user_id = match UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let first = form.first_name.trim();
    let last = form.last_name.trim();
    if first.is_empty() || last.is_empty() {
        return error_redirect("First and last name cannot be empty.")
            .into_response();
    }

    if db::users::update_name(&state.db, &user_id, first, last)
        .await
        .is_err()
    {
        return error_redirect("Failed to update name.").into_response();
    }

    Redirect::to("/app/account?success=name_updated").into_response()
}

/// POST /app/account/update-profile-image — Set or clear profile image URL.
pub async fn update_profile_image(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Form(form): Form<UpdateProfileImageForm>,
) -> impl IntoResponse {
    if let Err(ref errors) = form.validate() {
        return error_redirect(&errors.to_string()).into_response();
    }

    let user_id = match UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return Redirect::to("/login").into_response(),
    };

    let url: Option<ProfileImageUrl> = form
        .profile_image_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| ProfileImageUrl::new(s).expect("validation passed"));

    if db::users::update_profile_image_url(&state.db, &user_id, url.as_ref())
        .await
        .is_err()
    {
        return error_redirect("Failed to update profile image.").into_response();
    }

    Redirect::to("/app/account?success=profile_image_updated").into_response()
}

/// Account routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/app/account", get(show_account))
        .route("/app/account/change-password", post(change_password))
        .route("/app/account/update-name", post(update_name))
        .route("/app/account/update-profile-image", post(update_profile_image))
}
