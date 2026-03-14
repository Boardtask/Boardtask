use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use rand_core::RngCore;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};
use validator::Validate;

use crate::app::{
    db,
    domain::{Email, OrganizationId, OrganizationRole, UserId},
    session::AuthenticatedSession,
    tenant,
    AppState, APP_NAME,
};

/// One row for the members table on the org settings page.
pub struct MemberRow {
    pub display_name: String,
    pub email: String,
    pub role_display: String,
    pub avatar_url: String,
    pub initials: String,
}

/// One row for pending invites.
pub struct PendingInviteRow {
    pub id: String,
    pub email: String,
    pub role_display: String,
    pub sent_ago: String,
    pub expires_in: String,
}

/// Human-readable role label for the UI.
fn role_display(role: &str) -> String {
    match role.to_lowercase().as_str() {
        "owner" => "Owner".to_string(),
        "admin" => "Admin".to_string(),
        "member" => "Member (Read/Write)".to_string(),
        "viewer" => "Viewer".to_string(),
        _ => role.to_string(),
    }
}

/// Format "X days ago" or "X hours ago" from unix timestamp.
fn format_sent_ago(created_at: i64) -> String {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let diff = now - created_at;
    if diff < 3600 {
        let mins = diff / 60;
        if mins < 1 {
            "Just now".to_string()
        } else {
            format!("{} minutes ago", mins)
        }
    } else if diff < 86400 {
        format!("{} hours ago", diff / 3600)
    } else {
        format!("{} days ago", diff / 86400)
    }
}

/// Format "Expires in X days" from expires_at.
fn format_expires_in(expires_at: i64) -> String {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let diff = expires_at - now;
    if diff <= 0 {
        "Expired".to_string()
    } else if diff < 86400 {
        format!("Expires in {} hours", diff / 3600)
    } else {
        format!("Expires in {} days", diff / 86400)
    }
}

/// Initials from display name or email.
fn initials_from_name(name: &str, email: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() >= 2 {
        let first = parts[0].chars().next().unwrap_or('?');
        let last = parts[1].chars().next().unwrap_or('?');
        format!(
            "{}{}",
            first.to_uppercase().collect::<String>(),
            last.to_uppercase().collect::<String>()
        )
    } else if !name.is_empty() {
        name.chars().take(2).flat_map(|c| c.to_uppercase()).collect()
    } else {
        email.chars().take(2).flat_map(|c| c.to_uppercase()).collect()
    }
}

/// Organization management page template.
#[derive(Template)]
#[template(path = "organization_management.html")]
pub struct OrganizationSettingsTemplate {
    pub app_name: &'static str,
    pub org_name: String,
    pub members: Vec<MemberRow>,
    pub members_total: usize,
    pub pending_invites: Vec<PendingInviteRow>,
    pub error: String,
    pub success: String,
    pub current_user_avatar_url: String,
}

/// Invite form data from HTTP request.
#[derive(Debug, Deserialize, Validate)]
pub struct InviteForm {
    #[validate(length(min = 1, max = 254), email)]
    pub email: String,

    #[validate(length(min = 1))]
    pub role: String,
}

/// Query parameters for org settings page (error/success feedback).
#[derive(Debug, Deserialize)]
pub struct OrganizationSettingsQuery {
    pub error: Option<String>,
    pub success: Option<String>,
}

fn can_invite(role: OrganizationRole) -> bool {
    matches!(role, OrganizationRole::Owner | OrganizationRole::Admin)
}

/// GET /app/settings/organization — Show org name, members, pending invites, and invite form (owners/admins only).
pub async fn show(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Query(query): Query<OrganizationSettingsQuery>,
) -> Response {
    let role = match tenant::require_org_member(&state.db, &session.user_id, &session.organization_id).await {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response(),
    };

    if !can_invite(role) {
        return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response();
    }

    let org_id = match OrganizationId::from_string(&session.organization_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid organization".to_string()).into_response(),
    };

    let org = match db::organizations::find_by_id(&state.db, &org_id).await {
        Ok(Some(o)) => o,
        Ok(None) => return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()).into_response(),
    };

    let members = match db::organizations::list_members_with_email(&state.db, &org_id).await {
        Ok(m) => m,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()).into_response(),
    };
    let members: Vec<MemberRow> = members
        .into_iter()
        .map(|m| {
            let display_name = db::display_name_from_parts(&m.first_name, &m.last_name);
            let initials = initials_from_name(&display_name, &m.email);
            let avatar_url = m.profile_image_url.unwrap_or_default();
            MemberRow {
                display_name,
                email: m.email,
                role_display: role_display(&m.role),
                avatar_url,
                initials,
            }
        })
        .collect();

    let pending = match db::organization_invites::list_pending_for_org(&state.db, &org_id).await {
        Ok(p) => p,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()).into_response(),
    };
    let pending_invites: Vec<PendingInviteRow> = pending
        .into_iter()
        .map(|i| PendingInviteRow {
            id: i.id,
            email: i.email,
            role_display: role_display(&i.role),
            sent_ago: format_sent_ago(i.created_at),
            expires_in: format_expires_in(i.expires_at),
        })
        .collect();

    let user_id = match UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid session".to_string()).into_response(),
    };
    let current_user_avatar_url =
        db::users::profile_image_url_for(&state.db, &user_id).await;

    let members_total = members.len();
    let template = OrganizationSettingsTemplate {
        app_name: APP_NAME,
        org_name: org.name.clone(),
        members,
        members_total,
        pending_invites,
        error: query.error.unwrap_or_default(),
        success: query.success.unwrap_or_default(),
        current_user_avatar_url,
    };
    Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response()
}

fn invite_redirect_error(msg: &str) -> Redirect {
    Redirect::to(&format!("/app/settings/organization?error={}", urlencoding::encode(msg)))
}

fn invite_redirect_success(msg: &str) -> Redirect {
    Redirect::to(&format!("/app/settings/organization?success={}", urlencoding::encode(msg)))
}

/// Generate a high-entropy invite token (64 hex chars = 32 bytes).
fn generate_invite_token() -> String {
    let mut bytes = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// POST /app/settings/organization/invite — Create invite and send email (owners/admins only).
pub async fn create_invite(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Form(form): Form<InviteForm>,
) -> Response {
    let role = match tenant::require_org_member(&state.db, &session.user_id, &session.organization_id).await {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response(),
    };

    if !can_invite(role) {
        return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response();
    }

    if form.validate().is_err() {
        return invite_redirect_error("Invalid email or role.").into_response();
    }

    let email = match Email::new(form.email.clone()) {
        Ok(e) => e,
        Err(_) => return invite_redirect_error("Invalid email address.").into_response(),
    };
    let email_str = email.as_str().to_string();

    let invite_role = match form.role.parse::<OrganizationRole>() {
        Ok(r) => r,
        Err(_) => return invite_redirect_error("Invalid role.").into_response(),
    };

    let org_id = match OrganizationId::from_string(&session.organization_id) {
        Ok(id) => id,
        Err(_) => return invite_redirect_error("Invalid organization.").into_response(),
    };

    let user_id = match UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return invite_redirect_error("Invalid session.").into_response(),
    };

    // Already a member?
    if let Ok(Some(existing_user)) = db::find_by_email(&state.db, &email).await {
        let existing_id = UserId::from_string(&existing_user.id).unwrap_or_else(|_| UserId::new());
        if db::organizations::is_member(&state.db, &org_id, &existing_id).await.unwrap_or(false) {
            return invite_redirect_error("That email is already a member of this organization.").into_response();
        }
    }

    // Already a pending invite? Cancel it and we'll create a new one below.
    let mut resend = false;
    let existing = db::organization_invites::find_pending_by_org_and_email(&state.db, &org_id, &email_str)
        .await
        .unwrap_or(None);
    if let Some(existing_invite) = existing {
        if db::organization_invites::delete_by_id(&state.db, &existing_invite.id)
            .await
            .is_err()
        {
            return invite_redirect_error("Failed to update invite.").into_response();
        }
        resend = true;
    }

    let now = OffsetDateTime::now_utc().unix_timestamp();
    let expires_at = now + Duration::days(7).whole_seconds();
    let id = UserId::new().as_str();
    let token = generate_invite_token();

    let invite = db::organization_invites::NewOrganizationInvite {
        id: id.to_string(),
        organization_id: org_id.clone(),
        email: email_str.clone(),
        role: invite_role,
        invited_by_user_id: user_id,
        token: token.clone(),
        expires_at,
        created_at: now,
    };

    if db::organization_invites::insert(&state.db, &invite).await.is_err() {
        return invite_redirect_error("Failed to create invite.").into_response();
    }

    let invite_url = format!(
        "{}/accept-invite?token={}",
        state.config.app_url_base(),
        urlencoding::encode(&token)
    );
    let org = match db::organizations::find_by_id(&state.db, &org_id).await {
        Ok(Some(o)) => o,
        _ => return invite_redirect_error("Organization not found.").into_response(),
    };
    let body = format!(
        "You've been invited to join {} on {}. Click here to accept: {}",
        org.name,
        APP_NAME,
        invite_url
    );
    let msg = crate::app::mail::EmailMessage::new(
        email,
        format!("You're invited to join {}", org.name),
        body,
        state.config.mail_from.clone(),
    );
    if state.mail.send(&msg).await.is_err() {
        return invite_redirect_error("Invite created but we couldn't send the email. Please try again.").into_response();
    }

    let success_msg = if resend {
        "Previous invite cancelled; new invite sent."
    } else {
        "Invite sent."
    };
    invite_redirect_success(success_msg).into_response()
}

/// POST /app/settings/organization/invite/:id/revoke — Revoke a pending invite.
pub async fn revoke_invite(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Path(invite_id): Path<String>,
) -> Response {
    let role = match tenant::require_org_member(&state.db, &session.user_id, &session.organization_id).await {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response(),
    };
    if !can_invite(role) {
        return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response();
    }

    let invite = match db::organization_invites::find_by_id(&state.db, &invite_id).await {
        Ok(Some(i)) => i,
        _ => return invite_redirect_error("Invite not found.").into_response(),
    };
    if invite.organization_id != session.organization_id {
        return invite_redirect_error("Invite not found.").into_response();
    }

    if db::organization_invites::delete_by_id(&state.db, &invite_id).await.is_err() {
        return invite_redirect_error("Failed to revoke invite.").into_response();
    }
    invite_redirect_success("Invitation revoked.").into_response()
}

/// POST /app/settings/organization/invite/:id/resend — Resend invite email.
pub async fn resend_invite(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Path(invite_id): Path<String>,
) -> Response {
    let role = match tenant::require_org_member(&state.db, &session.user_id, &session.organization_id).await {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response(),
    };
    if !can_invite(role) {
        return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response();
    }

    let invite = match db::organization_invites::find_by_id(&state.db, &invite_id).await {
        Ok(Some(i)) => i,
        _ => return invite_redirect_error("Invite not found.").into_response(),
    };
    if invite.organization_id != session.organization_id {
        return invite_redirect_error("Invite not found.").into_response();
    }

    let org_id = match OrganizationId::from_string(&session.organization_id) {
        Ok(id) => id,
        Err(_) => return invite_redirect_error("Invalid organization.").into_response(),
    };
    let user_id = match UserId::from_string(&session.user_id) {
        Ok(id) => id,
        Err(_) => return invite_redirect_error("Invalid session.").into_response(),
    };
    let email = match Email::new(invite.email.clone()) {
        Ok(e) => e,
        Err(_) => return invite_redirect_error("Invalid invite email.").into_response(),
    };
    let invite_role = invite.role.parse::<OrganizationRole>().unwrap_or(OrganizationRole::Member);

    if db::organization_invites::delete_by_id(&state.db, &invite_id).await.is_err() {
        return invite_redirect_error("Failed to resend invite.").into_response();
    }

    let now = OffsetDateTime::now_utc().unix_timestamp();
    let expires_at = now + Duration::days(7).whole_seconds();
    let new_id = UserId::new().as_str();
    let token = generate_invite_token();

    let new_invite = db::organization_invites::NewOrganizationInvite {
        id: new_id.to_string(),
        organization_id: org_id.clone(),
        email: invite.email.clone(),
        role: invite_role,
        invited_by_user_id: user_id,
        token: token.clone(),
        expires_at,
        created_at: now,
    };

    if db::organization_invites::insert(&state.db, &new_invite).await.is_err() {
        return invite_redirect_error("Failed to resend invite.").into_response();
    }

    let org = match db::organizations::find_by_id(&state.db, &org_id).await {
        Ok(Some(o)) => o,
        _ => return invite_redirect_error("Organization not found.").into_response(),
    };
    let invite_url = format!(
        "{}/accept-invite?token={}",
        state.config.app_url_base(),
        urlencoding::encode(&token)
    );
    let body = format!(
        "You've been invited to join {} on {}. Click here to accept: {}",
        org.name,
        APP_NAME,
        invite_url
    );
    let msg = crate::app::mail::EmailMessage::new(
        email,
        format!("You're invited to join {}", org.name),
        body,
        state.config.mail_from.clone(),
    );
    if state.mail.send(&msg).await.is_err() {
        return invite_redirect_error("Invite resent but email could not be sent. Please try again.").into_response();
    }
    invite_redirect_success("Invitation resent.").into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/app/settings/organization", get(show))
        .route("/app/settings/organization/invite", post(create_invite))
        .route("/app/settings/organization/invite/:id/revoke", post(revoke_invite))
        .route("/app/settings/organization/invite/:id/resend", post(resend_invite))
}
