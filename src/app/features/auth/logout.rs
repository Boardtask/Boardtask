use axum::{
    extract::State,
    response::Redirect,
    routing::post, Router,
};
use axum_extra::extract::cookie::CookieJar;

use crate::app::{
    db,
    error::AppError,
    AppState,
};

/// POST /logout — Log out the current user.
pub async fn submit(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AppError> {
    if let Some(cookie) = jar.get("session_id") {
        let session_id = cookie.value();

        db::sessions::delete(&state.db, session_id)
            .await
            .map_err(AppError::Database)?;
    }

    let jar = jar.add(crate::app::session::clear_session_cookie());

    // Redirect to home
    Ok((jar, Redirect::to("/")))
}

/// Logout routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/logout", post(submit))
}