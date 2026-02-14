use axum_extra::extract::cookie::{Cookie, SameSite};

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
