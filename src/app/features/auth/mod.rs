pub mod signup;
pub mod login;
pub mod logout;

use axum::Router;
use crate::app::AppState;

/// Authentication routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(signup::routes())
        .merge(login::routes())
        .merge(logout::routes())
}