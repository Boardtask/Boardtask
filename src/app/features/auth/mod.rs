pub mod signup;
pub mod login;
pub mod logout;
pub mod verify_email;
pub mod password_reset;
pub mod resend_verification;

use axum::Router;
use crate::app::AppState;

/// Authentication routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(signup::routes())
        .merge(login::routes())
        .merge(logout::routes())
        .merge(verify_email::routes())
        .merge(password_reset::routes())
        .merge(resend_verification::routes())
}