//! Tests for project JSON export (GET) and import (POST) API.

use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

use crate::common::*;
use boardtask::app::db;

const TASK_NODE_TYPE_ID: &str = "01JNODETYPE00000000TASK000";
const DEFAULT_STATUS_ID: &str = "01JSTATUS00000000TODO0000";

mod export_tests {
    use super::*;

    #[tokio::test]
    async fn export_requires_authentication() {
        let pool = test_pool().await;
        let app = test_router(pool);

        let request = http::Request::builder()
            .method("GET")
            .uri("/api/projects/01JPROJECT000000000000001/export")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["error"], "Unauthorized");
    }

    #[tokio::test]
    async fn export_returns_401_without_valid_session() {
        let pool = test_pool().await;
        let app = test_router(pool);

        let request = http::Request::builder()
            .method("GET")
            .uri("/api/projects/01JPROJECT000000000000001/export")
            .header("cookie", "session_id=invalid")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn export_404_for_nonexistent_project() {
        let (cookie, _project_id, _pool, app, _) = setup_user_and_project("export404@example.com", "Password123").await;

        let request = http::Request::builder()
            .method("GET")
            .uri("/api/projects/01JPROJECT000000000000001/export")
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn export_succeeds_returns_json_with_attachment_header() {
        let (cookie, project_id, pool, app, _) = setup_user_and_project("export@example.com", "Password123").await;

        // Add a node and a slot so export has content
        let node_id = ulid::Ulid::new().to_string();
        let slot_id = ulid::Ulid::new().to_string();
        db::project_slots::insert(
            &pool,
            &db::NewProjectSlot {
                id: slot_id.clone(),
                project_id: project_id.clone(),
                name: "Sprint 1".to_string(),
                sort_order: 0,
            },
        )
        .await
        .unwrap();
        db::nodes::insert(
            &pool,
            &db::nodes::NewNode {
                id: node_id.clone(),
                project_id: project_id.clone(),
                node_type_id: TASK_NODE_TYPE_ID.to_string(),
                status_id: DEFAULT_STATUS_ID.to_string(),
                title: "Export me".to_string(),
                description: Some("Desc".to_string()),
                estimated_minutes: Some(60),
                slot_id: Some(slot_id),
                parent_id: None,
                assigned_user_id: None,
            },
        )
        .await
        .unwrap();

        let request = http::Request::builder()
            .method("GET")
            .uri(&format!("/api/projects/{}/export", project_id))
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").and_then(|v| v.to_str().ok()),
            Some("application/json")
        );
        let disposition = response
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            disposition.contains("attachment") && disposition.contains(".json"),
            "Expected Content-Disposition attachment with .json, got: {}",
            disposition
        );

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["version"], 1);
        assert_eq!(body["project"]["title"], "Test Project");
        assert!(body["slots"].is_array());
        assert_eq!(body["slots"].as_array().unwrap().len(), 1);
        assert_eq!(body["slots"][0]["name"], "Sprint 1");
        assert!(body["nodes"].is_array());
        assert_eq!(body["nodes"].as_array().unwrap().len(), 1);
        assert_eq!(body["nodes"][0]["title"], "Export me");
        assert_eq!(body["nodes"][0]["description"], "Desc");
        assert_eq!(body["nodes"][0]["estimated_minutes"], 60);
        assert!(body["edges"].is_array());
    }
}

mod import_tests {
    use super::*;

    fn valid_import_body() -> serde_json::Value {
        serde_json::json!({
            "version": 1,
            "project": { "title": "Imported Project" },
            "slots": [
                { "id": "01JSLOT00000000000000001", "name": "Slot A", "sort_order": 0 }
            ],
            "nodes": [
                {
                    "id": "01JNODE00000000000000001",
                    "node_type_id": TASK_NODE_TYPE_ID,
                    "status_id": DEFAULT_STATUS_ID,
                    "title": "Imported task",
                    "description": "Imported description",
                    "estimated_minutes": 90,
                    "slot_id": "01JSLOT00000000000000001",
                    "parent_id": null
                }
            ],
            "edges": []
        })
    }

    #[tokio::test]
    async fn import_requires_authentication() {
        let pool = test_pool().await;
        let app = test_router(pool.clone());
        ensure_graph_seeds(&pool).await;

        let body = valid_import_body();
        let request = http::Request::builder()
            .method("POST")
            .uri("/api/projects/import")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn import_returns_401_without_valid_session() {
        let pool = test_pool().await;
        let app = test_router(pool.clone());
        ensure_graph_seeds(&pool).await;

        let body = valid_import_body();
        let request = http::Request::builder()
            .method("POST")
            .uri("/api/projects/import")
            .header("content-type", "application/json")
            .header("cookie", "session_id=invalid")
            .body(axum::body::Body::from(body.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn import_validation_empty_title_returns_400() {
        let (cookie, _project_id, pool, app, _) = setup_user_and_project("importval@example.com", "Password123").await;
        ensure_graph_seeds(&pool).await;

        let body = serde_json::json!({
            "version": 1,
            "project": { "title": "   " },
            "slots": [],
            "nodes": [],
            "edges": []
        });

        let request = http::Request::builder()
            .method("POST")
            .uri("/api/projects/import")
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(body.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let res_body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(res_body["error"].as_str().unwrap().to_lowercase().contains("title"));
    }

    #[tokio::test]
    async fn import_validation_wrong_version_returns_400() {
        let (cookie, _project_id, pool, app, _) = setup_user_and_project("importver@example.com", "Password123").await;
        ensure_graph_seeds(&pool).await;

        let body = serde_json::json!({
            "version": 99,
            "project": { "title": "Valid Title" },
            "slots": [],
            "nodes": [],
            "edges": []
        });

        let request = http::Request::builder()
            .method("POST")
            .uri("/api/projects/import")
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(body.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let res_body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(res_body["error"].as_str().unwrap().to_lowercase().contains("version"));
    }

    #[tokio::test]
    async fn import_succeeds_creates_new_project_and_redirects() {
        let (cookie, _project_id, pool, app, _) = setup_user_and_project("importok@example.com", "Password123").await;
        ensure_graph_seeds(&pool).await;

        let body = valid_import_body();
        let request = http::Request::builder()
            .method("POST")
            .uri("/api/projects/import")
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(body.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            http::StatusCode::SEE_OTHER,
            "Expected 303 redirect on successful import"
        );

        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            location.starts_with("/app/projects/") && location.len() > "/app/projects/".len(),
            "Expected Location to be /app/projects/{{id}}, got: {}",
            location
        );

        let new_project_id = location.trim_start_matches("/app/projects/");
        let org_id = {
            let user_id = user_id_from_cookie(&pool, &cookie).await;
            let user = db::users::find_by_id(
                &pool,
                &boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
            )
            .await
            .unwrap()
            .expect("user exists");
            user.organization_id
        };

        let project = db::projects::find_by_id_and_org(&pool, new_project_id, &org_id)
            .await
            .unwrap()
            .expect("new project should exist");
        assert_eq!(project.title, "Imported Project");

        let nodes = db::nodes::find_by_project(&pool, new_project_id).await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].title, "Imported task");
        assert_eq!(nodes[0].description.as_deref(), Some("Imported description"));
        assert_eq!(nodes[0].estimated_minutes, Some(90));

        let slots = db::project_slots::find_by_project(&pool, new_project_id).await.unwrap();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].name, "Slot A");
    }
}
