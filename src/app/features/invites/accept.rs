use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use crate::app::{
    db,
    domain::{Email, OrganizationId, OrganizationRole, UserId},
    session::{self, AuthenticatedSession},
    AppState, APP_NAME,
};

/// Query for GET /accept-invite.
#[derive(Debug, Deserialize)]
pub struct AcceptInviteQuery {
    pub token: Option<String>,
}

/// Accept invite page: invalid/expired, new user (signup/login links), or existing user login prompt.
#[derive(Template)]
#[template(path = "accept_invite.html")]
pub struct AcceptInviteTemplate {
    pub app_name: &'static str,
    /// Set when invite is invalid or expired.
    pub invalid_message: String,
    /// Set when new user: org name and links to signup/login.
    pub new_user_org_name: String,
    pub new_user_email: String,
    pub new_user_signup_url: String,
    pub new_user_login_url: String,
    /// Set when existing user should log in.
    pub existing_org_name: String,
    pub existing_login_url: String,
}

#[derive(Debug, Clone)]
pub enum AcceptInviteState {
    InvalidExpired { message: String },
    NewUser {
        org_name: String,
        email: String,
        signup_url: String,
        login_url: String,
    },
    ExistingUser {
        org_name: String,
        login_url: String,
    },
}

impl AcceptInviteTemplate {
    fn from_state(app_name: &'static str, state: AcceptInviteState) -> Self {
        match state {
            AcceptInviteState::InvalidExpired { message } => AcceptInviteTemplate {
                app_name,
                invalid_message: message,
                new_user_org_name: String::new(),
                new_user_email: String::new(),
                new_user_signup_url: String::new(),
                new_user_login_url: String::new(),
                existing_org_name: String::new(),
                existing_login_url: String::new(),
            },
            AcceptInviteState::NewUser {
                org_name,
                email,
                signup_url,
                login_url,
            } => AcceptInviteTemplate {
                app_name,
                invalid_message: String::new(),
                new_user_org_name: org_name,
                new_user_email: email,
                new_user_signup_url: signup_url,
                new_user_login_url: login_url,
                existing_org_name: String::new(),
                existing_login_url: String::new(),
            },
            AcceptInviteState::ExistingUser {
                org_name,
                login_url,
            } => AcceptInviteTemplate {
                app_name,
                invalid_message: String::new(),
                new_user_org_name: String::new(),
                new_user_email: String::new(),
                new_user_signup_url: String::new(),
                new_user_login_url: String::new(),
                existing_org_name: org_name,
                existing_login_url: login_url,
            },
        }
    }
}

/// GET /accept-invite — Show invite state: error, signup/login links (new user), or login link (existing user).
pub async fn show(
    State(state): State<AppState>,
    Query(query): Query<AcceptInviteQuery>,
) -> Response {
    let token = match &query.token {
        Some(t) if !t.is_empty() => t.clone(),
        _ => {
            let tmpl = AcceptInviteTemplate::from_state(
                APP_NAME,
                AcceptInviteState::InvalidExpired {
                    message: "Invalid or missing invite link.".to_string(),
                },
            );
            return Html(tmpl.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
        }
    };

    let invite = match db::organization_invites::find_by_token(&state.db, &token).await {
        Ok(Some(inv)) => inv,
        Ok(None) => {
            let tmpl = AcceptInviteTemplate::from_state(
                APP_NAME,
                AcceptInviteState::InvalidExpired {
                    message: "This invite is invalid or has expired. Ask your teammate to send a new one.".to_string(),
                },
            );
            return Html(tmpl.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
        }
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let org_id = match OrganizationId::from_string(&invite.organization_id) {
        Ok(id) => id,
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Invalid organization".to_string()).into_response(),
    };
    let org = match db::organizations::find_by_id(&state.db, &org_id).await {
        Ok(Some(o)) => o,
        Ok(None) => {
            let tmpl = AcceptInviteTemplate::from_state(
                APP_NAME,
                AcceptInviteState::InvalidExpired {
                    message: "This invite is no longer valid.".to_string(),
                },
            );
            return Html(tmpl.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
        }
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let email_for_lookup = match Email::new(invite.email.clone()) {
        Ok(e) => e,
        Err(_) => {
            let tmpl = AcceptInviteTemplate::from_state(
                APP_NAME,
                AcceptInviteState::InvalidExpired {
                    message: "Invalid invite.".to_string(),
                },
            );
            return Html(tmpl.render().unwrap_or_else(|_| "Template error".to_string())).into_response();
        }
    };

    let existing_user = db::find_by_email(&state.db, &email_for_lookup).await.ok().flatten();
    let state_enum = if existing_user.is_some() {
        let login_url = format!(
            "/login?next={}",
            urlencoding::encode(&format!("/accept-invite/confirm?token={}", urlencoding::encode(&token)))
        );
        AcceptInviteState::ExistingUser {
            org_name: org.name,
            login_url,
        }
    } else {
        let login_url = format!(
            "/login?next={}",
            urlencoding::encode(&format!("/accept-invite/confirm?token={}", urlencoding::encode(&token)))
        );
        let signup_url = format!(
            "/signup?email={}&next={}",
            urlencoding::encode(&invite.email),
            urlencoding::encode(&format!("/accept-invite/confirm?token={}", urlencoding::encode(&token)))
        );
        AcceptInviteState::NewUser {
            org_name: org.name,
            email: invite.email,
            signup_url,
            login_url,
        }
    };

    let tmpl = AcceptInviteTemplate::from_state(APP_NAME, state_enum);
    Html(tmpl.render().unwrap_or_else(|_| "Template error".to_string())).into_response()
}


/// Query for GET /accept-invite/confirm.
#[derive(Debug, Deserialize)]
pub struct ConfirmInviteQuery {
    pub token: Option<String>,
}

/// GET /accept-invite/confirm — After login, consume invite: add user to org, switch active org, new session. Requires auth.
pub async fn confirm_existing_user(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Query(query): Query<ConfirmInviteQuery>,
    jar: CookieJar,
) -> Response {
    let token = match &query.token {
        Some(t) if !t.is_empty() => t.clone(),
        _ => return Redirect::to("/app").into_response(),
    };

    let invite = match db::organization_invites::find_by_token(&state.db, &token).await {
        Ok(Some(inv)) => inv,
        Ok(None) => return Redirect::to("/accept-invite").into_response(),
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()).into_response(),
    };

    let user_id = match UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return Redirect::to("/app").into_response(),
    };
    let user = match db::users::find_by_id(&state.db, &user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return Redirect::to("/app").into_response(),
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()).into_response(),
    };

    if user.email.to_lowercase() != invite.email.to_lowercase() {
        return Redirect::to(&format!("/accept-invite?token={}", urlencoding::encode(&token))).into_response();
    }

    let org_id = match OrganizationId::from_string(&invite.organization_id) {
        Ok(id) => id,
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Invalid organization".to_string()).into_response(),
    };
    let role = invite.role.parse::<OrganizationRole>().unwrap_or(OrganizationRole::Member);

    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()).into_response(),
    };

    if !db::organizations::is_member(&mut *tx, &org_id, &user_id).await.unwrap_or(false) {
        if db::organizations::add_member(&mut *tx, &org_id, &user_id, role).await.is_err() {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to add to organization".to_string()).into_response();
        }
    }
    // Add user to org's default team (idempotent)
    if let Ok(Some(default_team)) = db::teams::find_default_for_org(&mut *tx, &org_id).await {
        let _ = db::team_members::add_member(&mut *tx, &default_team.id, &user_id).await;
    }
    if db::users::update_organization_id(&mut *tx, &user_id, &org_id).await.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to update organization".to_string()).into_response();
    }
    if db::organization_invites::delete_by_id(&mut *tx, &invite.id).await.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to complete invite".to_string()).into_response();
    }
    if tx.commit().await.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()).into_response();
    }

    if db::sessions::delete(&state.db, &session.id).await.is_err() {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to update session".to_string()).into_response();
    }

    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);
    let new_session_id = match db::sessions::create(&state.db, &user_id, &org_id, expires_at).await {
        Ok(s) => s,
        Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to create session".to_string()).into_response(),
    };

    let jar = jar.add(session::session_cookie(new_session_id));
    (jar, Redirect::to("/app")).into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/accept-invite", get(show))
        .route("/accept-invite/confirm", get(confirm_existing_user))
}
