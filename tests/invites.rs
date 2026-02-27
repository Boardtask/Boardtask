//! Integration tests for organization invite flows.

use http_body_util::BodyExt;
use time::OffsetDateTime;
use tower::ServiceExt;

mod common;

use crate::common::*;

fn invite_form_body(email: &str, role: &str) -> String {
    format!(
        "email={}&role={}",
        urlencoding::encode(email),
        urlencoding::encode(role)
    )
}

fn signup_form_body_with_next(email: &str, password: &str, confirm_password: &str, next: &str) -> String {
    format!(
        "email={}&password={}&confirm_password={}&next={}",
        urlencoding::encode(email),
        urlencoding::encode(password),
        urlencoding::encode(confirm_password),
        urlencoding::encode(next)
    )
}

#[tokio::test]
async fn organization_settings_requires_authentication() {
    let pool = test_pool().await;
    let app = test_router(pool);

    let request = http::Request::builder()
        .method("GET")
        .uri("/app/settings/organization")
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers().get("location").map(|v| v.to_str().unwrap()),
        Some("/login")
    );
}

#[tokio::test]
async fn create_invite_as_owner_succeeds_and_creates_db_row() {
    let (cookie, _project_id, pool, app, _) = setup_user_and_project("inviter@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(
        &pool,
        &boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
    )
    .await
    .unwrap()
    .expect("user exists");
    let org_id = boardtask::app::domain::OrganizationId::from_string(&user.organization_id).unwrap();

    let body = invite_form_body("newuser@example.com", "member");
    let request = http::Request::builder()
        .method("POST")
        .uri("/app/settings/organization/invite")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(body))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);
    let location = response.headers().get("location").and_then(|v| v.to_str().ok()).unwrap_or("");
    assert!(
        location.contains("/app/settings/organization"),
        "Expected redirect to org settings, got: {}",
        location
    );
    assert!(location.contains("success="), "Expected success param, got: {}", location);

    let pending = boardtask::app::db::organization_invites::list_pending_for_org(&pool, &org_id)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].email, "newuser@example.com");
    assert_eq!(pending[0].role, "member");
}

#[tokio::test]
async fn accept_invite_new_user_get_shows_signup_link_no_password_form() {
    let (cookie, _project_id, pool, app, _) = setup_user_and_project("owner@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(
        &pool,
        &boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
    )
    .await
    .unwrap()
    .expect("user exists");
    let org_id = boardtask::app::domain::OrganizationId::from_string(&user.organization_id).unwrap();

    let invite_id = boardtask::app::domain::UserId::new().as_str();
    let token = boardtask::app::domain::UserId::new().as_str();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let invite = boardtask::app::db::organization_invites::NewOrganizationInvite {
        id: invite_id.to_string(),
        organization_id: org_id.clone(),
        email: "newbie@example.com".to_string(),
        role: boardtask::app::domain::OrganizationRole::Member,
        invited_by_user_id: boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
        token: token.to_string(),
        expires_at: now + 86400 * 7,
        created_at: now,
    };
    boardtask::app::db::organization_invites::insert(&pool, &invite).await.unwrap();

    let request = http::Request::builder()
        .method("GET")
        .uri(&format!("/accept-invite?token={}", urlencoding::encode(&token)))
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains("/signup?"),
        "Expected signup URL in body, got: {}",
        body_str
    );
    assert!(
        body_str.contains("newbie@example.com") || body_str.contains("newbie%40example.com"),
        "Expected invite email in signup link, got: {}",
        body_str
    );
    assert!(
        body_str.contains("accept-invite/confirm") || body_str.contains("accept-invite%2Fconfirm"),
        "Expected next=confirm in signup link, got: {}",
        body_str
    );
    assert!(
        !body_str.contains("name=\"password\""),
        "Expected no password form for new user, got: {}",
        body_str
    );
}

#[tokio::test]
async fn accept_invite_post_returns_method_not_allowed() {
    let pool = test_pool().await;
    let app = test_router(pool);

    let body = "token=abc&password=Password123&confirm_password=Password123";
    let request = http::Request::builder()
        .method("POST")
        .uri("/accept-invite")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(body))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn accept_invite_as_new_user_creates_user_and_redirects_to_app() {
    let (cookie, _project_id, pool, app, _) = setup_user_and_project("owner@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(
        &pool,
        &boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
    )
    .await
    .unwrap()
    .expect("user exists");
    let org_id = boardtask::app::domain::OrganizationId::from_string(&user.organization_id).unwrap();

    let invite_id = boardtask::app::domain::UserId::new().as_str();
    let token = boardtask::app::domain::UserId::new().as_str();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let expires_at = now + 86400 * 7;
    let invite = boardtask::app::db::organization_invites::NewOrganizationInvite {
        id: invite_id.to_string(),
        organization_id: org_id.clone(),
        email: "newbie@example.com".to_string(),
        role: boardtask::app::domain::OrganizationRole::Member,
        invited_by_user_id: boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
        token: token.to_string(),
        expires_at,
        created_at: now,
    };
    boardtask::app::db::organization_invites::insert(&pool, &invite).await.unwrap();

    // New flow: signup with next → verify-email → confirm
    let next = format!("/accept-invite/confirm?token={}", urlencoding::encode(&token));
    let signup_body = signup_form_body_with_next("newbie@example.com", "Password123", "Password123", &next);
    let signup_request = http::Request::builder()
        .method("POST")
        .uri("/signup")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(signup_body))
        .unwrap();
    let signup_response = app.clone().oneshot(signup_request).await.unwrap();
    assert_eq!(signup_response.status(), http::StatusCode::SEE_OTHER);
    let location = signup_response.headers().get("location").and_then(|v| v.to_str().ok()).unwrap_or("");
    assert!(location.contains("/check-email"), "Expected redirect to check-email, got: {}", location);

    let email = boardtask::app::domain::Email::new("newbie@example.com".to_string()).unwrap();
    let new_user = boardtask::app::db::find_by_email(&pool, &email).await.unwrap().expect("user created by signup");
    let new_user_id = boardtask::app::domain::UserId::from_string(&new_user.id).unwrap();
    let verify_token = boardtask::app::db::email_verification::find_token_for_user(&pool, &new_user_id)
        .await
        .unwrap()
        .expect("verification token exists");

    let verify_url = format!(
        "/verify-email?token={}&next={}",
        verify_token,
        urlencoding::encode(&format!("/accept-invite/confirm?token={}", urlencoding::encode(&token)))
    );
    let verify_request = http::Request::builder()
        .method("GET")
        .uri(&verify_url)
        .body(axum::body::Body::empty())
        .unwrap();
    let verify_response = app.clone().oneshot(verify_request).await.unwrap();
    assert_eq!(verify_response.status(), http::StatusCode::SEE_OTHER);
    let verify_location = verify_response.headers().get("location").and_then(|v| v.to_str().ok()).unwrap_or("");
    assert!(
        verify_location.contains("accept-invite/confirm"),
        "Expected redirect to confirm, got: {}",
        verify_location
    );
    let set_cookie = verify_response.headers().get("set-cookie").expect("Set-Cookie").to_str().unwrap();
    let new_cookie = set_cookie.split(';').next().unwrap_or("").to_string();

    let confirm_request = http::Request::builder()
        .method("GET")
        .uri(verify_location.to_string())
        .header("cookie", &new_cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let confirm_response = app.clone().oneshot(confirm_request).await.unwrap();
    assert_eq!(confirm_response.status(), http::StatusCode::SEE_OTHER);
    assert_eq!(
        confirm_response.headers().get("location").map(|v| v.to_str().unwrap()),
        Some("/app")
    );

    let new_user_after = boardtask::app::db::find_by_email(&pool, &email).await.unwrap().expect("user exists");
    assert_eq!(new_user_after.organization_id, org_id.as_str());

    let is_member = boardtask::app::db::organizations::is_member(&pool, &org_id, &new_user_id).await.unwrap();
    assert!(is_member);

    let invite_still = boardtask::app::db::organization_invites::find_by_token(&pool, &token).await.unwrap();
    assert!(invite_still.is_none(), "invite should be consumed");
}

#[tokio::test]
async fn accept_invite_as_new_user_then_app_accessible() {
    let (cookie, project_id, pool, app, _) = setup_user_and_project("owner2@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(
        &pool,
        &boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
    )
    .await
    .unwrap()
    .expect("user exists");
    let org_id = boardtask::app::domain::OrganizationId::from_string(&user.organization_id).unwrap();

    let invite_id = boardtask::app::domain::UserId::new().as_str();
    let token = boardtask::app::domain::UserId::new().as_str();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let invite = boardtask::app::db::organization_invites::NewOrganizationInvite {
        id: invite_id.to_string(),
        organization_id: org_id.clone(),
        email: "teammate@example.com".to_string(),
        role: boardtask::app::domain::OrganizationRole::Member,
        invited_by_user_id: boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
        token: token.to_string(),
        expires_at: now + 86400 * 7,
        created_at: now,
    };
    boardtask::app::db::organization_invites::insert(&pool, &invite).await.unwrap();

    // New flow: signup with next → verify-email → confirm
    let next = format!("/accept-invite/confirm?token={}", urlencoding::encode(&token));
    let signup_body = signup_form_body_with_next("teammate@example.com", "Password123", "Password123", &next);
    let signup_request = http::Request::builder()
        .method("POST")
        .uri("/signup")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(signup_body))
        .unwrap();
    let signup_response = app.clone().oneshot(signup_request).await.unwrap();
    assert_eq!(signup_response.status(), http::StatusCode::SEE_OTHER);

    let email = boardtask::app::domain::Email::new("teammate@example.com".to_string()).unwrap();
    let new_user = boardtask::app::db::find_by_email(&pool, &email).await.unwrap().expect("user created by signup");
    let new_user_id = boardtask::app::domain::UserId::from_string(&new_user.id).unwrap();
    let verify_token = boardtask::app::db::email_verification::find_token_for_user(&pool, &new_user_id)
        .await
        .unwrap()
        .expect("verification token exists");

    let verify_url = format!(
        "/verify-email?token={}&next={}",
        verify_token,
        urlencoding::encode(&format!("/accept-invite/confirm?token={}", urlencoding::encode(&token)))
    );
    let verify_request = http::Request::builder()
        .method("GET")
        .uri(&verify_url)
        .body(axum::body::Body::empty())
        .unwrap();
    let verify_response = app.clone().oneshot(verify_request).await.unwrap();
    assert_eq!(verify_response.status(), http::StatusCode::SEE_OTHER);
    let verify_location = verify_response.headers().get("location").and_then(|v| v.to_str().ok()).unwrap_or("");
    let set_cookie = verify_response.headers().get("set-cookie").expect("Set-Cookie").to_str().unwrap();
    let new_cookie = set_cookie.split(';').next().unwrap_or("").to_string();

    let confirm_request = http::Request::builder()
        .method("GET")
        .uri(verify_location.to_string())
        .header("cookie", &new_cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let confirm_response = app.oneshot(confirm_request).await.unwrap();
    assert_eq!(confirm_response.status(), http::StatusCode::SEE_OTHER);

    let cookie_after_confirm = confirm_response
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or("").to_string())
        .unwrap_or(new_cookie);

    let app2 = test_router(pool.clone());
    let list_request = http::Request::builder()
        .method("GET")
        .uri("/app/projects")
        .header("cookie", &cookie_after_confirm)
        .body(axum::body::Body::empty())
        .unwrap();
    let list_response = app2.clone().oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), http::StatusCode::OK);

    let list_body_bytes = list_response.into_body().collect().await.unwrap().to_bytes();
    let list_body_str = String::from_utf8_lossy(&list_body_bytes);
    assert!(
        list_body_str.contains("Test Project"),
        "Invited org member should see owner's project in list, got: {}",
        list_body_str
    );

    let show_request = http::Request::builder()
        .method("GET")
        .uri(&format!("/app/projects/{}", project_id))
        .header("cookie", &cookie_after_confirm)
        .body(axum::body::Body::empty())
        .unwrap();
    let show_response = app2.oneshot(show_request).await.unwrap();
    assert_eq!(show_response.status(), http::StatusCode::OK);
}

#[tokio::test]
async fn accept_invite_as_existing_user_switches_org() {
    let (owner_cookie, owner_project_id, pool, app, _) = setup_user_and_project("orgowner@example.com", "Password123").await;
    let owner_user_id = user_id_from_cookie(&pool, &owner_cookie).await;
    let owner_user = boardtask::app::db::users::find_by_id(
        &pool,
        &boardtask::app::domain::UserId::from_string(&owner_user_id).unwrap(),
    )
    .await
    .unwrap()
    .expect("owner exists");
    let org_id = boardtask::app::domain::OrganizationId::from_string(&owner_user.organization_id).unwrap();

    let (invitee_user_id, _invitee_email, _invitee_password) =
        create_verified_user(&pool, "existing@example.com", "Password123").await;
    let invitee_before = boardtask::app::db::users::find_by_id(&pool, &invitee_user_id).await.unwrap().expect("invitee exists");
    let invitee_org_before = invitee_before.organization_id.clone();

    let invite_id = boardtask::app::domain::UserId::new().as_str();
    let token = boardtask::app::domain::UserId::new().as_str();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let invite = boardtask::app::db::organization_invites::NewOrganizationInvite {
        id: invite_id.to_string(),
        organization_id: org_id.clone(),
        email: "existing@example.com".to_string(),
        role: boardtask::app::domain::OrganizationRole::Member,
        invited_by_user_id: boardtask::app::domain::UserId::from_string(&owner_user_id).unwrap(),
        token: token.to_string(),
        expires_at: now + 86400 * 7,
        created_at: now,
    };
    boardtask::app::db::organization_invites::insert(&pool, &invite).await.unwrap();

    let login_body = login_form_body("existing@example.com", "Password123");
    let login_request = http::Request::builder()
        .method("POST")
        .uri("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(login_body))
        .unwrap();
    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), http::StatusCode::SEE_OTHER);
    let set_cookie = login_response.headers().get("set-cookie").unwrap().to_str().unwrap();
    let invitee_cookie = set_cookie.split(';').next().unwrap_or("").to_string();

    let confirm_url = format!("/accept-invite/confirm?token={}", urlencoding::encode(&token));
    let confirm_request = http::Request::builder()
        .method("GET")
        .uri(&confirm_url)
        .header("cookie", &invitee_cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let confirm_response = app.clone().oneshot(confirm_request).await.unwrap();
    assert_eq!(confirm_response.status(), http::StatusCode::SEE_OTHER);
    assert_eq!(
        confirm_response.headers().get("location").map(|v| v.to_str().unwrap()),
        Some("/app")
    );

    let invitee_after = boardtask::app::db::users::find_by_id(&pool, &invitee_user_id).await.unwrap().expect("invitee exists");
    assert_eq!(invitee_after.organization_id, org_id.as_str());
    assert_ne!(invitee_after.organization_id, invitee_org_before);

    let is_member = boardtask::app::db::organizations::is_member(&pool, &org_id, &invitee_user_id).await.unwrap();
    assert!(is_member);

    let new_cookie = confirm_response.headers().get("set-cookie").and_then(|v| v.to_str().ok()).map(|s| s.split(';').next().unwrap_or("").to_string());
    let cookie_to_use = new_cookie.as_deref().unwrap_or(&invitee_cookie);
    let show_request = http::Request::builder()
        .method("GET")
        .uri(&format!("/app/projects/{}", owner_project_id))
        .header("cookie", cookie_to_use)
        .body(axum::body::Body::empty())
        .unwrap();
    let show_response = app.oneshot(show_request).await.unwrap();
    assert_eq!(show_response.status(), http::StatusCode::OK);
}

/// Regression: accept-invite/confirm must rotate session inside the same transaction as the user org update.
/// If session delete/create were done after commit and one of them failed, the user would have
/// user.organization_id = new org but session.organization_id = old org, breaking permission checks.
#[tokio::test]
async fn accept_invite_as_existing_user_session_and_user_org_stay_in_sync() {
    let (owner_cookie, _owner_project_id, pool, app, _) = setup_user_and_project("orgowner2@example.com", "Password123").await;
    let owner_user_id = user_id_from_cookie(&pool, &owner_cookie).await;
    let owner_user = boardtask::app::db::users::find_by_id(
        &pool,
        &boardtask::app::domain::UserId::from_string(&owner_user_id).unwrap(),
    )
    .await
    .unwrap()
    .expect("owner exists");
    let invited_org_id = boardtask::app::domain::OrganizationId::from_string(&owner_user.organization_id).unwrap();

    let (invitee_user_id, _invitee_email, _invitee_password) =
        create_verified_user(&pool, "sync@example.com", "Password123").await;
    let invitee_before = boardtask::app::db::users::find_by_id(&pool, &invitee_user_id).await.unwrap().expect("invitee exists");
    let _invitee_org_before = invitee_before.organization_id.clone();

    let invite_id = boardtask::app::domain::UserId::new().as_str();
    let token = boardtask::app::domain::UserId::new().as_str();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let invite = boardtask::app::db::organization_invites::NewOrganizationInvite {
        id: invite_id.to_string(),
        organization_id: invited_org_id.clone(),
        email: "sync@example.com".to_string(),
        role: boardtask::app::domain::OrganizationRole::Member,
        invited_by_user_id: boardtask::app::domain::UserId::from_string(&owner_user_id).unwrap(),
        token: token.to_string(),
        expires_at: now + 86400 * 7,
        created_at: now,
    };
    boardtask::app::db::organization_invites::insert(&pool, &invite).await.unwrap();

    let login_body = login_form_body("sync@example.com", "Password123");
    let login_request = http::Request::builder()
        .method("POST")
        .uri("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(login_body))
        .unwrap();
    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), http::StatusCode::SEE_OTHER);
    let set_cookie = login_response.headers().get("set-cookie").unwrap().to_str().unwrap();
    let invitee_cookie = set_cookie.split(';').next().unwrap_or("").to_string();
    let old_session_id = extract_session_id_from_cookie(&invitee_cookie).expect("login sets session_id");

    let confirm_url = format!("/accept-invite/confirm?token={}", urlencoding::encode(&token));
    let confirm_request = http::Request::builder()
        .method("GET")
        .uri(&confirm_url)
        .header("cookie", &invitee_cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let confirm_response = app.oneshot(confirm_request).await.unwrap();
    assert_eq!(confirm_response.status(), http::StatusCode::SEE_OTHER);
    assert_eq!(
        confirm_response.headers().get("location").map(|v| v.to_str().unwrap()),
        Some("/app")
    );

    // New session must be returned in Set-Cookie
    let set_cookie_after = confirm_response
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .expect("confirm must set new session cookie");
    let new_session_id = extract_session_id_from_cookie(set_cookie_after).expect("cookie contains session_id");
    assert_ne!(old_session_id, new_session_id, "session must be rotated (new id)");

    // Old session must be deleted (otherwise we could have committed user org update without rotating session)
    let old_session_still_valid = boardtask::app::db::sessions::find_valid(&pool, old_session_id).await.unwrap();
    assert!(
        old_session_still_valid.is_none(),
        "old session must be deleted so user and session never point to different orgs"
    );

    // New session must exist and have the invited org
    let new_session = boardtask::app::db::sessions::find_valid(&pool, new_session_id).await.unwrap();
    let new_session = new_session.expect("new session must exist after confirm");
    assert_eq!(
        new_session.organization_id,
        invited_org_id.as_str(),
        "new session must have organization_id = invited org"
    );

    // User record must match
    let invitee_after = boardtask::app::db::users::find_by_id(&pool, &invitee_user_id).await.unwrap().expect("invitee exists");
    assert_eq!(
        invitee_after.organization_id,
        invited_org_id.as_str(),
        "user.organization_id must equal invited org"
    );
    assert_eq!(
        invitee_after.organization_id,
        new_session.organization_id,
        "user and session must always point to the same org after accept-invite"
    );
}

#[tokio::test]
async fn accept_invite_invalid_token_shows_error() {
    let pool = test_pool().await;
    let app = test_router(pool);

    let request = http::Request::builder()
        .method("GET")
        .uri("/accept-invite?token=invalid-token-xyz")
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains("invalid") || body_str.contains("expired"),
        "Expected invalid/expired message, got: {}",
        body_str
    );
}

#[tokio::test]
async fn accept_invite_expired_token_shows_error() {
    let (cookie, _project_id, pool, app, _) = setup_user_and_project("expirer@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(
        &pool,
        &boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
    )
    .await
    .unwrap()
    .expect("user exists");
    let org_id = boardtask::app::domain::OrganizationId::from_string(&user.organization_id).unwrap();

    let invite_id = boardtask::app::domain::UserId::new().as_str();
    let token = boardtask::app::domain::UserId::new().as_str();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let invite = boardtask::app::db::organization_invites::NewOrganizationInvite {
        id: invite_id.to_string(),
        organization_id: org_id.clone(),
        email: "expired@example.com".to_string(),
        role: boardtask::app::domain::OrganizationRole::Member,
        invited_by_user_id: boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
        token: token.to_string(),
        expires_at: now - 1,
        created_at: now - 86400,
    };
    boardtask::app::db::organization_invites::insert(&pool, &invite).await.unwrap();

    let request = http::Request::builder()
        .method("GET")
        .uri(&format!("/accept-invite?token={}", urlencoding::encode(&token)))
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains("invalid") || body_str.contains("expired"),
        "Expected invalid/expired message for expired token, got: {}",
        body_str
    );
}
