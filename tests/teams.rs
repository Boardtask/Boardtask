//! Integration tests for `/app/teams` and `/app/teams/:team_id`.

use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

use boardtask::app::db;
use boardtask::app::domain::OrganizationId;

use crate::common::*;

#[tokio::test]
async fn list_teams_requires_authentication() {
    let pool = common::test_pool().await;
    let app = common::test_router(pool);

    let request = http::Request::builder()
        .method("GET")
        .uri("/app/teams")
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
async fn team_show_requires_authentication() {
    let pool = common::test_pool().await;
    let app = common::test_router(pool);

    let team_id = ulid::Ulid::new().to_string();
    let request = http::Request::builder()
        .method("GET")
        .uri(&format!("/app/teams/{}", team_id))
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
async fn list_teams_includes_link_to_team_detail() {
    let (cookie, _project_id, _pool, app, team_id) =
        setup_user_and_project("teamslist@example.com", "Password123").await;

    let request = http::Request::builder()
        .method("GET")
        .uri("/app/teams")
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    let expected_href = format!("/app/teams/{}", team_id);
    assert!(
        body_str.contains(&expected_href),
        "Expected teams list to link to {}, got fragment missing. Body len {}",
        expected_href,
        body_str.len()
    );
    assert!(
        body_str.contains("Teams"),
        "Expected Teams heading in list, got: {}",
        body_str
    );
}

#[tokio::test]
async fn team_show_404_for_nonexistent_team() {
    let (cookie, _project_id, _pool, app, _) =
        setup_user_and_project("team404@example.com", "Password123").await;

    let fake_id = "01HZ9999999999999999999999";
    let request = http::Request::builder()
        .method("GET")
        .uri(&format!("/app/teams/{}", fake_id))
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn team_show_renders_members_for_default_team() {
    let (cookie, _project_id, _pool, app, team_id) =
        setup_user_and_project("teamshow@example.com", "Password123").await;

    let request = http::Request::builder()
        .method("GET")
        .uri(&format!("/app/teams/{}", team_id))
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);

    assert!(
        body_str.contains("Back to teams"),
        "Expected back link, got: {}",
        body_str
    );
    assert!(
        body_str.contains("Test Org"),
        "Expected default team name 'Test Org' in page, got: {}",
        body_str
    );
    assert!(
        body_str.contains("Members"),
        "Expected Members section, got: {}",
        body_str
    );
    assert!(
        body_str.contains("1 Total"),
        "Expected member count header, got: {}",
        body_str
    );
    assert!(
        body_str.contains("teamshow@example.com"),
        "Expected member email in table, got: {}",
        body_str
    );
    assert!(
        body_str.contains("Test User") || body_str.contains("teamshow@example.com"),
        "Expected display name or email for member, got: {}",
        body_str
    );
}

#[tokio::test]
async fn team_show_404_when_team_belongs_to_another_organization() {
    let (cookie_a, _project_id, pool, app, _team_a) =
        setup_user_and_project("orga@example.com", "Password123").await;

    let (user_b_id, _, _) = create_verified_user(&pool, "orgb@example.com", "Password123").await;
    let user_b = db::users::find_by_id(&pool, &user_b_id)
        .await
        .unwrap()
        .expect("user b exists");
    let org_b = OrganizationId::from_string(&user_b.organization_id).unwrap();
    let team_b = db::teams::find_default_for_org(&pool, &org_b)
        .await
        .unwrap()
        .expect("org b has default team");

    let request = http::Request::builder()
        .method("GET")
        .uri(&format!("/app/teams/{}", team_b.id))
        .header("cookie", &cookie_a)
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        http::StatusCode::NOT_FOUND,
        "Must not leak other org teams; expect 404"
    );
}
