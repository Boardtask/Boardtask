use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

use crate::common::*;

#[tokio::test]
async fn create_form_requires_authentication() {
    let pool = common::test_pool().await;
    let app = common::test_router(pool);

    let request = http::Request::builder()
        .method("GET")
        .uri("/app/projects/new")
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
async fn create_post_requires_authentication() {
    let pool = common::test_pool().await;
    let app = common::test_router(pool);

    let body = create_project_form_body("My Project", "dummy");
    let request = http::Request::builder()
        .method("POST")
        .uri("/app/projects")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(body))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers().get("location").map(|v| v.to_str().unwrap()),
        Some("/login")
    );
}

#[tokio::test]
async fn create_project_succeeds() {
    let (cookie, _project_id, _pool, app, team_id) = setup_user_and_project("create@example.com", "Password123").await;

    let body = create_project_form_body("My Project", &team_id);
    let request = http::Request::builder()
        .method("POST")
        .uri("/app/projects")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(body))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers().get("location").map(|v| v.to_str().unwrap()),
        Some("/app/projects")
    );

    // Verify project appears in list
    let list_request = http::Request::builder()
        .method("GET")
        .uri("/app/projects")
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let list_response = app.oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), http::StatusCode::OK);

    let body_bytes = list_response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains("My Project"),
        "Expected 'My Project' in list, got: {}",
        body_str
    );
}

#[tokio::test]
async fn create_project_empty_title_returns_error() {
    let (cookie, _project_id, _pool, app, team_id) = setup_user_and_project("empty@example.com", "Password123").await;

    let body = create_project_form_body("", &team_id);
    let request = http::Request::builder()
        .method("POST")
        .uri("/app/projects")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(body))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains("1") && body_str.contains("255"),
        "Expected validation error about title length, got: {}",
        body_str
    );
}

#[tokio::test]
async fn list_projects_requires_authentication() {
    let pool = common::test_pool().await;
    let app = common::test_router(pool);

    let request = http::Request::builder()
        .method("GET")
        .uri("/app/projects")
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
async fn list_projects_shows_org_projects() {
    let (cookie, _project_id, pool, app, team_id) = setup_user_and_project("list@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
    let org_id = user.organization_id.clone();

    use boardtask::app::db::projects;
    let project1 = projects::NewProject {
        id: ulid::Ulid::new().to_string(),
        title: "Project Alpha".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
        team_id: team_id.clone(),
    };
    let project2 = projects::NewProject {
        id: ulid::Ulid::new().to_string(),
        title: "Project Beta".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
        team_id,
    };
    projects::insert(&pool, &project1).await.unwrap();
    projects::insert(&pool, &project2).await.unwrap();

    let request = http::Request::builder()
        .method("GET")
        .uri("/app/projects")
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains("Project Alpha") && body_str.contains("Project Beta"),
        "Expected both org project titles in list, got: {}",
        body_str
    );
}

#[tokio::test]
async fn list_projects_shows_enriched_metadata() {
    let (cookie, _project_id, _pool, app, _team_id) = setup_user_and_project("enriched@example.com", "Password123").await;

    let request = http::Request::builder()
        .method("GET")
        .uri("/app/projects")
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);

    // New design: Active Projects title, project title, task count, progress, blockers, Create New Project
    assert!(
        body_str.contains("Active Projects"),
        "Expected 'Active Projects' in list, got: {}",
        body_str
    );
    assert!(
        body_str.contains("Test Project"),
        "Expected project title in list, got: {}",
        body_str
    );
    assert!(
        body_str.contains("Create New Project"),
        "Expected 'Create New Project' in list, got: {}",
        body_str
    );
    assert!(
        body_str.contains("Global Efficiency"),
        "Expected footer stats in list, got: {}",
        body_str
    );
}

#[tokio::test]
async fn show_project_requires_authentication() {
    let pool = common::test_pool().await;
    let app = common::test_router(pool);

    let project_id = ulid::Ulid::new().to_string();
    let request = http::Request::builder()
        .method("GET")
        .uri(&format!("/app/projects/{}", project_id))
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
async fn show_project_renders() {
    let (cookie, project_id, _pool, app, _) = setup_user_and_project("show@example.com", "Password123").await;

    let request = http::Request::builder()
        .method("GET")
        .uri(&format!("/app/projects/{}", project_id))
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains("Test Project") && body_str.contains(&project_id),
        "Expected project title and id in body, got: {}",
        body_str
    );
}

#[tokio::test]
async fn show_project_404_for_nonexistent() {
    let (cookie, _project_id, _pool, app, _) = setup_user_and_project("404@example.com", "Password123").await;

    let nonexistent_id = "01HZ9999999999999999999999";
    let request = http::Request::builder()
        .method("GET")
        .uri(&format!("/app/projects/{}", nonexistent_id))
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn patch_project_settings_requires_authentication() {
    let pool = common::test_pool().await;
    let app = common::test_router(pool);

    let project_id = ulid::Ulid::new().to_string();
    let body = r#"{"default_view_mode":"list"}"#;
    let request = http::Request::builder()
        .method("PATCH")
        .uri(&format!("/api/projects/{}", project_id))
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn patch_project_settings_invalid_default_view_mode_returns_error() {
    let (cookie, project_id, _pool, app, _) =
        setup_user_and_project("patch_invalid@example.com", "Password123").await;

    let body = r#"{"default_view_mode":"invalid"}"#;
    let request = http::Request::builder()
        .method("PATCH")
        .uri(&format!("/api/projects/{}", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(body))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    // 422 Unprocessable Entity for JSON deserialization failure, or 400 for validation
    assert!(
        response.status() == http::StatusCode::BAD_REQUEST
            || response.status() == http::StatusCode::UNPROCESSABLE_ENTITY,
        "Expected 400 or 422 for invalid default_view_mode, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn patch_project_settings_updates_and_returns() {
    let (cookie, project_id, _pool, app, _) =
        setup_user_and_project("patch_ok@example.com", "Password123").await;

    let body = r#"{"default_view_mode":"list"}"#;
    let request = http::Request::builder()
        .method("PATCH")
        .uri(&format!("/api/projects/{}", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(body))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains(r#""default_view_mode":"list""#),
        "Expected default_view_mode in response, got: {}",
        body_str
    );

    // GET /app/projects list should render href with /list for this project
    let list_request = http::Request::builder()
        .method("GET")
        .uri("/app/projects")
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let list_response = app.oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), http::StatusCode::OK);
    let list_bytes = list_response.into_body().collect().await.unwrap().to_bytes();
    let list_str = String::from_utf8_lossy(&list_bytes);
    assert!(
        list_str.contains(&format!("/app/projects/{}/list", project_id)),
        "Expected project list href to point to list view, got: {}",
        list_str
    );
}

#[tokio::test]
async fn patch_project_settings_tenant_isolation() {
    let (_owner_cookie, project_id, pool, app, _) =
        setup_user_and_project("patch_owner@example.com", "Password123").await;
    create_verified_user(&pool, "patch_other@example.com", "Password123").await;

    let login_body = login_form_body("patch_other@example.com", "Password123");
    let login_request = http::Request::builder()
        .method("POST")
        .uri("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(login_body))
        .unwrap();
    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), http::StatusCode::SEE_OTHER);
    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let session_id = extract_session_id_from_cookie(set_cookie).expect("login sets session_id");
    let other_cookie = format!("session_id={}", session_id);

    let body = r#"{"default_view_mode":"list"}"#;
    let request = http::Request::builder()
        .method("PATCH")
        .uri(&format!("/api/projects/{}", project_id))
        .header("content-type", "application/json")
        .header("cookie", &other_cookie)
        .body(axum::body::Body::from(body))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        http::StatusCode::NOT_FOUND,
        "User in different org must not patch project"
    );
}

#[tokio::test]
async fn show_project_404_when_user_in_different_org() {
    let (_owner_cookie, project_id, pool, app, _) = setup_user_and_project("owner@example.com", "Password123").await;
    create_verified_user(&pool, "other@example.com", "Password123").await;

    let login_body = login_form_body("other@example.com", "Password123");
    let login_request = http::Request::builder()
        .method("POST")
        .uri("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(login_body))
        .unwrap();
    let login_response = app.clone().oneshot(login_request).await.unwrap();
    assert_eq!(login_response.status(), http::StatusCode::SEE_OTHER);
    let set_cookie = login_response.headers().get("set-cookie").unwrap().to_str().unwrap();
    let other_cookie = set_cookie.split(';').next().unwrap_or("").to_string();

    let request = http::Request::builder()
        .method("GET")
        .uri(&format!("/app/projects/{}", project_id))
        .header("cookie", &other_cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        http::StatusCode::NOT_FOUND,
        "User in different org must not access project"
    );
}
