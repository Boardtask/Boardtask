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

    let body = create_project_form_body("My Project");
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
    let pool = common::test_pool().await;
    let app = common::test_router(pool.clone());
    let cookie = authenticated_cookie(&pool, &app, "create@example.com", "Password123").await;

    let body = create_project_form_body("My Project");
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
    let pool = common::test_pool().await;
    let app = common::test_router(pool.clone());
    let cookie = authenticated_cookie(&pool, &app, "empty@example.com", "Password123").await;

    let body = create_project_form_body("");
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
async fn list_projects_shows_user_projects() {
    let pool = common::test_pool().await;
    let app = common::test_router(pool.clone());
    let cookie = authenticated_cookie(&pool, &app, "list@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;

    use boardtask::app::db::projects;
    let project1 = projects::NewProject {
        id: ulid::Ulid::new().to_string(),
        title: "Project Alpha".to_string(),
        user_id: user_id.clone(),
    };
    let project2 = projects::NewProject {
        id: ulid::Ulid::new().to_string(),
        title: "Project Beta".to_string(),
        user_id: user_id.clone(),
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
        "Expected both project titles in list, got: {}",
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
    let pool = common::test_pool().await;
    let app = common::test_router(pool.clone());
    let cookie = authenticated_cookie(&pool, &app, "show@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;

    let project_id = ulid::Ulid::new().to_string();
    let project_title = "My Awesome Project";
    use boardtask::app::db::projects;
    let new_project = projects::NewProject {
        id: project_id.clone(),
        title: project_title.to_string(),
        user_id: user_id.clone(),
    };
    projects::insert(&pool, &new_project).await.unwrap();

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
        body_str.contains(project_title) && body_str.contains(&project_id),
        "Expected project title and id in body, got: {}",
        body_str
    );
}

#[tokio::test]
async fn show_project_404_for_nonexistent() {
    let pool = common::test_pool().await;
    let app = common::test_router(pool.clone());
    let cookie = authenticated_cookie(&pool, &app, "404@example.com", "Password123").await;

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
