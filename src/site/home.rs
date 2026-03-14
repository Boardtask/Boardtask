use axum::{response::Redirect, routing::get, Router};

use crate::app::{session::OptionalAuthenticatedSession, AppState};

/// GET / — Redirect to /app if authenticated, else /login.
pub async fn index(
    OptionalAuthenticatedSession(session): OptionalAuthenticatedSession,
) -> Redirect {
    if session.is_some() {
        Redirect::to("/app")
    } else {
        Redirect::to("/login")
    }
}

/// Routes for the home feature slice.
pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(index))
}
