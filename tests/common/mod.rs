#![allow(dead_code)]

use axum::body::Body;
use boardtask::create_router;
use sqlx::SqlitePool;
use tower::ServiceExt;

pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

pub fn test_router(pool: SqlitePool) -> axum::Router {
    let state = boardtask::app::AppState {
        db: pool,
        mail: std::sync::Arc::new(boardtask::app::mail::ConsoleMailer),
        config: boardtask::app::config::Config::for_tests(),
        resend_cooldown: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
    };
    create_router(state)
}

pub async fn ensure_graph_seeds(pool: &SqlitePool) {
    boardtask::app::features::graph::sync_system_node_types(pool).await.unwrap();
}

pub fn signup_form_body(email: &str, password: &str, confirm_password: &str) -> String {
    format!(
        "email={}&password={}&confirm_password={}",
        urlencoding::encode(email),
        urlencoding::encode(password),
        urlencoding::encode(confirm_password)
    )
}

pub fn login_form_body(email: &str, password: &str) -> String {
    format!(
        "email={}&password={}",
        urlencoding::encode(email),
        urlencoding::encode(password)
    )
}

pub fn create_project_form_body(title: &str) -> String {
    format!("title={}", urlencoding::encode(title))
}

pub fn forgot_password_form_body(email: &str) -> String {
    format!("email={}", urlencoding::encode(email))
}

pub fn change_password_form_body(
    current_password: &str,
    new_password: &str,
    confirm_password: &str,
) -> String {
    format!(
        "current_password={}&new_password={}&confirm_password={}",
        urlencoding::encode(current_password),
        urlencoding::encode(new_password),
        urlencoding::encode(confirm_password)
    )
}

pub fn extract_session_id_from_cookie(set_cookie_header: &str) -> Option<&str> {
    set_cookie_header.split(';').next()?.strip_prefix("session_id=")
}

/// Get user_id from an authenticated cookie header (e.g. from authenticated_cookie).
pub async fn user_id_from_cookie(pool: &sqlx::SqlitePool, cookie: &str) -> String {
    let session_id = extract_session_id_from_cookie(cookie).expect("cookie must contain session_id");
    let session = boardtask::app::db::sessions::find_valid(pool, session_id)
        .await
        .unwrap()
        .expect("session should be valid");
    session.user_id
}

/// Seeds graph, creates a user (via authenticated_cookie), creates one project.
/// Returns (cookie, project_id, pool, app). Use pool for extra DB setup (e.g. nodes); use app for requests.
pub async fn setup_user_and_project(
    email: &str,
    password: &str,
) -> (String, String, SqlitePool, axum::Router) {
    use boardtask::app::db::{self, projects};

    let pool = test_pool().await;
    let app = test_router(pool.clone());
    ensure_graph_seeds(&pool).await;

    let cookie = authenticated_cookie(&pool, &app, email, password).await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = db::users::find_by_id(
        &pool,
        &boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
    )
    .await
    .unwrap()
    .expect("user should exist");

    let project_id = ulid::Ulid::new().to_string();
    let project = db::NewProject {
        id: project_id.clone(),
        title: "Test Project".to_string(),
        user_id: user_id.clone(),
        organization_id: user.organization_id.clone(),
    };
    projects::insert(&pool, &project).await.unwrap();

    (cookie, project_id, pool, app)
}

/// Create a verified user directly in the database (bypasses signup flow).
pub async fn create_verified_user(
    pool: &sqlx::SqlitePool,
    email: &str,
    password: &str,
) -> (
    boardtask::app::domain::UserId,
    boardtask::app::domain::Email,
    boardtask::app::domain::Password,
) {
    use boardtask::app::db;
    use boardtask::app::domain::{Email, HashedPassword, Password, UserId};

    let email_type = Email::new(email.to_string()).unwrap();
    let password_type = Password::new(password.to_string()).unwrap();
    let password_hash = HashedPassword::from_password(&password_type).unwrap();
    let user_id = UserId::new();

    // Create organization
    let org_id = boardtask::app::domain::OrganizationId::new();
    let org = boardtask::app::db::organizations::NewOrganization {
        id: org_id.clone(),
        name: "Test Org".to_string(),
    };
    boardtask::app::db::organizations::insert(pool, &org).await.unwrap();

    let new_user = boardtask::app::db::NewUser {
        id: user_id.clone(),
        email: email_type.clone(),
        password_hash,
        organization_id: org_id.clone(),
    };
    boardtask::app::db::users::insert(pool, &new_user).await.unwrap();

    // Add user to organization
    boardtask::app::db::organizations::add_member(
        pool,
        &org_id,
        &user_id,
        boardtask::app::domain::OrganizationRole::Owner,
    )
    .await
    .unwrap();

    db::mark_verified(pool, &user_id).await.unwrap();

    (user_id, email_type, password_type)
}

/// Create verified user, login, return cookie header for authenticated requests.
pub async fn authenticated_cookie(
    pool: &sqlx::SqlitePool,
    app: &axum::Router,
    email: &str,
    password: &str,
) -> String {
    use boardtask::app::db;
    use boardtask::app::domain::{Email, HashedPassword, Password, UserId};

    let email_type = Email::new(email.to_string()).unwrap();
    let password_type = Password::new(password.to_string()).unwrap();
    let password_hash = HashedPassword::from_password(&password_type).unwrap();
    let user_id = UserId::new();

    // Create organization
    let org_id = boardtask::app::domain::OrganizationId::new();
    let org = boardtask::app::db::organizations::NewOrganization {
        id: org_id.clone(),
        name: "Test Org".to_string(),
    };
    boardtask::app::db::organizations::insert(pool, &org).await.unwrap();

    let new_user = boardtask::app::db::NewUser {
        id: user_id.clone(),
        email: email_type.clone(),
        password_hash,
        organization_id: org_id.clone(),
    };
    boardtask::app::db::users::insert(pool, &new_user).await.unwrap();

    // Add user to organization
    boardtask::app::db::organizations::add_member(
        pool,
        &org_id,
        &user_id,
        boardtask::app::domain::OrganizationRole::Owner,
    )
    .await
    .unwrap();

    db::mark_verified(pool, &user_id).await.unwrap();

    let login_body = login_form_body(email, password);
    let login_request = http::Request::builder()
        .method("POST")
        .uri("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(login_body))
        .unwrap();
    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), http::StatusCode::SEE_OTHER);

    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let session_id = extract_session_id_from_cookie(set_cookie).unwrap();
    format!("session_id={}", session_id)
}
