use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    response::{Json, Redirect},
    Json as JsonResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde_json::json;

use crate::app::{db, AppState};

/// Extractor that validates the session cookie and loads the session.
/// Rejects with a redirect to `/login` if the session is missing or invalid.
#[derive(Debug, Clone)]
pub struct AuthenticatedSession(pub db::sessions::Session);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedSession
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = Redirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| Redirect::to("/login"))?;

        let session_id = jar
            .get("session_id")
            .map(|c: &Cookie| c.value().to_string())
            .ok_or(Redirect::to("/login"))?;

        let app_state = AppState::from_ref(state);
        let session = db::sessions::find_valid(&app_state.db, &session_id)
            .await
            .map_err(|_| Redirect::to("/login"))?
            .ok_or(Redirect::to("/login"))?;

        Ok(AuthenticatedSession(session))
    }
}

pub fn session_cookie(session_id: impl Into<String>) -> Cookie<'static> {
    Cookie::build(("session_id", session_id.into()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .build()
}

pub fn clear_session_cookie() -> Cookie<'static> {
    Cookie::build(("session_id", ""))
        .path("/")
        .removal()
        .into()
}

/// Extractor that validates the session cookie and loads the session.
/// Rejects with JSON 401 instead of redirect for API use.
#[derive(Debug, Clone)]
pub struct ApiAuthenticatedSession(pub db::sessions::Session);

#[async_trait]
impl<S> FromRequestParts<S> for ApiAuthenticatedSession
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = (StatusCode, JsonResponse<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))))?;

        let session_id = jar
            .get("session_id")
            .map(|c| c.value().to_string())
            .ok_or((StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))))?;

        let app_state = AppState::from_ref(state);
        let session = db::sessions::find_valid(&app_state.db, &session_id)
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))))?
            .ok_or((StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))))?;

        Ok(ApiAuthenticatedSession(session))
    }
}
