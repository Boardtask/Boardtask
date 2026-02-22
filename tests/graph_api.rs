use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

use crate::common::*;
use boardtask::app::db;

const TASK_NODE_TYPE_ID: &str = "01JNODETYPE00000000TASK000";
const DEFAULT_STATUS_ID: &str = "01JSTATUS00000000TODO0000";
const STATUS_ID_IN_PROGRESS: &str = "01JSTATUS00000000INPROG00";
const STATUS_ID_DONE: &str = "01JSTATUS00000000DONE0000";

mod nodes {
    use super::*;

    #[tokio::test]
    async fn post_node_requires_authentication() {
    let pool = test_pool().await;
    let app = test_router(pool);

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Test Node"
    });

    let request = http::Request::builder()
        .method("POST")
        .uri("/api/projects/123/nodes")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Unauthorized");
}

#[tokio::test]
async fn post_node_returns_401_without_valid_session() {
    let pool = test_pool().await;
    let app = test_router(pool);

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Test Node"
    });

    let request = http::Request::builder()
        .method("POST")
        .uri("/api/projects/123/nodes")
        .header("content-type", "application/json")
        .header("cookie", "session_id=invalid")
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Unauthorized");
}

#[tokio::test]
async fn post_node_succeeds() {
    let (cookie, project_id, _pool, app) = setup_user_and_project("node@example.com", "Password123").await;

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Test Node",
        "description": "A test node",
        "estimated_minutes": 30
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::CREATED);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["project_id"], project_id);
    assert_eq!(body["node_type_id"], TASK_NODE_TYPE_ID);
    assert_eq!(body["title"], "Test Node");
    assert_eq!(body["description"], "A test node");
    assert_eq!(body["estimated_minutes"], 30);
    assert!(body["id"].is_string());
    assert!(body["created_at"].is_number());
}

#[tokio::test]
async fn post_node_404_for_nonexistent_project() {
    let pool = test_pool().await;
    let app = test_router(pool.clone());
    ensure_graph_seeds(&pool).await;

    let cookie = authenticated_cookie(&pool, &app, "nonexistent@example.com", "Password123").await;

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Test Node"
    });

    let nonexistent_project_id = "01HZ9999999999999999999999";
    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", nonexistent_project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::NOT_FOUND);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Project not found");

    let post_slot_req = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/slots", nonexistent_project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(serde_json::json!({ "name": "FE 1" }).to_string()))
        .unwrap();
    let slot_res = app.oneshot(post_slot_req).await.unwrap();
    assert_eq!(slot_res.status(), http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn post_node_404_for_project_owned_by_other_user() {
    let (_cookie_a, project_id, pool, app) = setup_user_and_project("usera@example.com", "Password123").await;
    let cookie_b = authenticated_cookie(&pool, &app, "userb@example.com", "Password123").await;

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Test Node"
    });

    // User B tries to create node in User A's project
    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie_b)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::NOT_FOUND);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Project not found");
}

#[tokio::test]
async fn post_node_invalid_node_type_returns_error() {
    let (cookie, project_id, _pool, app) = setup_user_and_project("invalidtype@example.com", "Password123").await;

    let request_body = serde_json::json!({
        "node_type_id": "invalid-node-type-id",
        "title": "Test Node"
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Invalid node_type_id");
}

#[tokio::test]
async fn post_node_invalid_status_id_returns_error() {
    let (cookie, project_id, _pool, app) = setup_user_and_project("invalidstatus@example.com", "Password123").await;

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Test Node",
        "status_id": "invalid-status-id"
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Invalid status_id");
}

#[tokio::test]
async fn post_node_with_slot_id_succeeds() {
    let (cookie, project_id, _pool, app) = setup_user_and_project("nodeslot@example.com", "Password123").await;

    let post_slot_req = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/slots", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(serde_json::json!({ "name": "FE 1" }).to_string()))
        .unwrap();
    let post_slot_res = app.clone().oneshot(post_slot_req).await.unwrap();
    assert_eq!(post_slot_res.status(), http::StatusCode::CREATED);
    let slot_body: serde_json::Value = serde_json::from_slice(&post_slot_res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let slot_id = slot_body["id"].as_str().unwrap();

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Node with slot",
        "slot_id": slot_id
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::CREATED);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["slot_id"], slot_id);
}

#[tokio::test]
async fn post_node_invalid_slot_id_returns_error() {
    let (cookie, project_id, _pool, app) = setup_user_and_project("invslot@example.com", "Password123").await;

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Test Node",
        "slot_id": "nonexistent-slot-id"
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Invalid slot_id");
}

#[tokio::test]
async fn post_node_slot_id_from_other_project_returns_error() {
    let (cookie, project_id, pool, app) = setup_user_and_project("otherslot@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
    let org_id = user.organization_id.clone();

    let other_project_id = ulid::Ulid::new().to_string();
    let other_project = db::NewProject {
        id: other_project_id.clone(),
        title: "Other Project".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
    };
    boardtask::app::db::projects::insert(&pool, &other_project).await.unwrap();

    let slot_id = ulid::Ulid::new().to_string();
    let slot = db::project_slots::NewProjectSlot {
        id: slot_id.clone(),
        project_id: other_project_id.clone(),
        name: "FE 1".to_string(),
        sort_order: 0,
    };
    boardtask::app::db::project_slots::insert(&pool, &slot).await.unwrap();

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Test Node",
        "slot_id": slot_id
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Invalid slot_id");
}

#[tokio::test]
async fn post_node_with_parent_id_succeeds() {
    let (cookie, project_id, _pool, app) = setup_user_and_project("parentid@example.com", "Password123").await;

    let post_group_req = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(
            serde_json::json!({ "node_type_id": TASK_NODE_TYPE_ID, "title": "Group" }).to_string(),
        ))
        .unwrap();
    let post_group_res = app.clone().oneshot(post_group_req).await.unwrap();
    assert_eq!(post_group_res.status(), http::StatusCode::CREATED);
    let group_body: serde_json::Value =
        serde_json::from_slice(&post_group_res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let group_id = group_body["id"].as_str().unwrap();

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Child in group",
        "parent_id": group_id
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::CREATED);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["parent_id"], group_id);
    let child_id = body["id"].as_str().unwrap();

    let get_request = http::Request::builder()
        .method("GET")
        .uri(&format!("/api/projects/{}/graph", project_id))
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let get_response = app.oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), http::StatusCode::OK);
    let get_bytes = get_response.into_body().collect().await.unwrap().to_bytes();
    let graph: serde_json::Value = serde_json::from_slice(&get_bytes).unwrap();
    let nodes = graph["nodes"].as_array().unwrap();
    let child = nodes.iter().find(|n| n["id"] == child_id).unwrap();
    assert_eq!(child["parent_id"], group_id);
}

#[tokio::test]
async fn post_node_invalid_parent_id_returns_error() {
    let (cookie, project_id, _pool, app) = setup_user_and_project("invparent@example.com", "Password123").await;

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Test Node",
        "parent_id": "nonexistent-id"
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Invalid parent_id");
}

#[tokio::test]
async fn post_node_parent_id_from_other_project_returns_error() {
    let (cookie, project_id, pool, app) = setup_user_and_project("otherparent@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
    let org_id = user.organization_id.clone();

    let other_project_id = ulid::Ulid::new().to_string();
    let other_project = db::NewProject {
        id: other_project_id.clone(),
        title: "Other Project".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
    };
    boardtask::app::db::projects::insert(&pool, &other_project).await.unwrap();

    let other_node_id = ulid::Ulid::new().to_string();
    let other_node = db::nodes::NewNode {
        id: other_node_id.clone(),
        project_id: other_project_id.clone(),
        node_type_id: TASK_NODE_TYPE_ID.to_string(),
        status_id: DEFAULT_STATUS_ID.to_string(),
        title: "Other Node".to_string(),
        description: None,
        estimated_minutes: None,
        slot_id: None,
        parent_id: None,
    };
    boardtask::app::db::nodes::insert(&pool, &other_node).await.unwrap();

    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Test Node",
        "parent_id": other_node_id
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Invalid parent_id");
}

#[tokio::test]
async fn post_node_with_status_then_patch_and_get_graph() {
    let (cookie, project_id, _pool, app) = setup_user_and_project("statusflow@example.com", "Password123").await;

    // Create node with explicit "In progress" status
    let request_body = serde_json::json!({
        "node_type_id": TASK_NODE_TYPE_ID,
        "title": "Status Test Node",
        "status_id": STATUS_ID_IN_PROGRESS
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/nodes", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::CREATED);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let node_id = body["id"].as_str().unwrap();
    assert_eq!(body["status_id"], STATUS_ID_IN_PROGRESS);

    // PATCH to "Done"
    let patch_body = serde_json::json!({ "status_id": STATUS_ID_DONE });
    let patch_request = http::Request::builder()
        .method("PATCH")
        .uri(&format!("/api/projects/{}/nodes/{}", project_id, node_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(patch_body.to_string()))
        .unwrap();

    let patch_response = app.clone().oneshot(patch_request).await.unwrap();
    assert_eq!(patch_response.status(), http::StatusCode::OK);

    let patch_res_bytes = patch_response.into_body().collect().await.unwrap().to_bytes();
    let patch_res: serde_json::Value = serde_json::from_slice(&patch_res_bytes).unwrap();
    assert_eq!(patch_res["status_id"], STATUS_ID_DONE);

    // GET graph and assert node has status_id Done
    let get_request = http::Request::builder()
        .method("GET")
        .uri(&format!("/api/projects/{}/graph", project_id))
        .header("cookie", &cookie)
        .body(axum::body::Body::empty())
        .unwrap();

    let get_response = app.clone().oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), http::StatusCode::OK);

    let get_bytes = get_response.into_body().collect().await.unwrap().to_bytes();
    let graph: serde_json::Value = serde_json::from_slice(&get_bytes).unwrap();
    let nodes = graph["nodes"].as_array().unwrap();
    let node = nodes.iter().find(|n| n["id"] == node_id).unwrap();
    assert_eq!(node["status_id"], STATUS_ID_DONE);
    }
}

mod patch_node {
    use super::*;

    /// PATCH with estimated_minutes: null clears the estimate and persists (custom deserializer preserves null vs omit).
    #[tokio::test]
    async fn patch_node_clearing_estimated_minutes_persists() {
        let (cookie, project_id, pool, app) = setup_user_and_project("patchclear@example.com", "Password123").await;

        let node_id = ulid::Ulid::new().to_string();
        let node = db::nodes::NewNode {
            id: node_id.clone(),
            project_id: project_id.clone(),
            node_type_id: TASK_NODE_TYPE_ID.to_string(),
            status_id: DEFAULT_STATUS_ID.to_string(),
            title: "Node With Estimate".to_string(),
            description: None,
            estimated_minutes: Some(30),
            slot_id: None,
            parent_id: None,
        };
        boardtask::app::db::nodes::insert(&pool, &node).await.unwrap();

        // Clear estimated_minutes by sending explicit null
        let request_body = serde_json::json!({
            "estimated_minutes": null
        });

        let request = http::Request::builder()
            .method("PATCH")
            .uri(&format!("/api/projects/{}/nodes/{}", project_id, node_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(request_body.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::OK);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(body["estimated_minutes"].is_null(), "PATCH response should have null estimated_minutes");

        // Verify persistence: GET graph and assert the node has null estimated_minutes
        let get_request = http::Request::builder()
            .method("GET")
            .uri(&format!("/api/projects/{}/graph", project_id))
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();

        let get_response = app.oneshot(get_request).await.unwrap();
        assert_eq!(get_response.status(), http::StatusCode::OK);

        let get_body_bytes = get_response.into_body().collect().await.unwrap().to_bytes();
        let get_body: serde_json::Value = serde_json::from_slice(&get_body_bytes).unwrap();
        let nodes = get_body["nodes"].as_array().unwrap();
        let updated = nodes.iter().find(|n| n["id"] == node_id).unwrap();
        assert!(
            updated["estimated_minutes"].is_null(),
            "Graph should return node with null estimated_minutes after clear"
        );
    }

    #[tokio::test]
    async fn patch_node_set_slot_id_then_clear() {
        let (cookie, project_id, pool, app) = setup_user_and_project("patchslot@example.com", "Password123").await;

        let post_slot_req = http::Request::builder()
            .method("POST")
            .uri(&format!("/api/projects/{}/slots", project_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(serde_json::json!({ "name": "BE 1" }).to_string()))
            .unwrap();
        let post_slot_res = app.clone().oneshot(post_slot_req).await.unwrap();
        assert_eq!(post_slot_res.status(), http::StatusCode::CREATED);
        let slot_body: serde_json::Value = serde_json::from_slice(&post_slot_res.into_body().collect().await.unwrap().to_bytes()).unwrap();
        let slot_id = slot_body["id"].as_str().unwrap().to_string();

        let patch_slot_req = http::Request::builder()
            .method("PATCH")
            .uri(&format!("/api/projects/{}/slots/{}", project_id, slot_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(serde_json::json!({ "name": "Back-end Developer 1" }).to_string()))
            .unwrap();
        let patch_slot_res = app.clone().oneshot(patch_slot_req).await.unwrap();
        assert_eq!(patch_slot_res.status(), http::StatusCode::OK);
        let patch_slot_body: serde_json::Value = serde_json::from_slice(&patch_slot_res.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(patch_slot_body["name"], "Back-end Developer 1");

        let node_id = ulid::Ulid::new().to_string();
        let node = db::nodes::NewNode {
            id: node_id.clone(),
            project_id: project_id.clone(),
            node_type_id: TASK_NODE_TYPE_ID.to_string(),
            status_id: DEFAULT_STATUS_ID.to_string(),
            title: "Node".to_string(),
            description: None,
            estimated_minutes: None,
            slot_id: None,
            parent_id: None,
        };
        boardtask::app::db::nodes::insert(&pool, &node).await.unwrap();

        let patch_body = serde_json::json!({ "slot_id": slot_id });
        let patch_request = http::Request::builder()
            .method("PATCH")
            .uri(&format!("/api/projects/{}/nodes/{}", project_id, node_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(patch_body.to_string()))
            .unwrap();

        let patch_response = app.clone().oneshot(patch_request).await.unwrap();
        assert_eq!(patch_response.status(), http::StatusCode::OK);

        let patch_res_bytes = patch_response.into_body().collect().await.unwrap().to_bytes();
        let patch_res: serde_json::Value = serde_json::from_slice(&patch_res_bytes).unwrap();
        assert_eq!(patch_res["slot_id"], slot_id);

        let get_request = http::Request::builder()
            .method("GET")
            .uri(&format!("/api/projects/{}/graph", project_id))
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();
        let get_response = app.clone().oneshot(get_request).await.unwrap();
        assert_eq!(get_response.status(), http::StatusCode::OK);
        let get_bytes = get_response.into_body().collect().await.unwrap().to_bytes();
        let graph: serde_json::Value = serde_json::from_slice(&get_bytes).unwrap();
        let nodes = graph["nodes"].as_array().unwrap();
        let updated_node = nodes.iter().find(|n| n["id"] == node_id).unwrap();
        assert_eq!(updated_node["slot_id"], slot_id);

        let clear_body = serde_json::json!({ "slot_id": null });
        let clear_request = http::Request::builder()
            .method("PATCH")
            .uri(&format!("/api/projects/{}/nodes/{}", project_id, node_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(clear_body.to_string()))
            .unwrap();

        let clear_response = app.clone().oneshot(clear_request).await.unwrap();
        assert_eq!(clear_response.status(), http::StatusCode::OK);

        let clear_res_bytes = clear_response.into_body().collect().await.unwrap().to_bytes();
        let clear_res: serde_json::Value = serde_json::from_slice(&clear_res_bytes).unwrap();
        assert!(clear_res["slot_id"].is_null());

        let get_request2 = http::Request::builder()
            .method("GET")
            .uri(&format!("/api/projects/{}/graph", project_id))
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();
        let get_response2 = app.clone().oneshot(get_request2).await.unwrap();
        let get_bytes2 = get_response2.into_body().collect().await.unwrap().to_bytes();
        let graph2: serde_json::Value = serde_json::from_slice(&get_bytes2).unwrap();
        let nodes2 = graph2["nodes"].as_array().unwrap();
        let cleared_node = nodes2.iter().find(|n| n["id"] == node_id).unwrap();
        assert!(cleared_node["slot_id"].is_null());

        // Re-assign slot then DELETE slot; node's slot_id should be cleared
        let set_slot_req = http::Request::builder()
            .method("PATCH")
            .uri(&format!("/api/projects/{}/nodes/{}", project_id, node_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(serde_json::json!({ "slot_id": slot_id }).to_string()))
            .unwrap();
        let _ = app.clone().oneshot(set_slot_req).await.unwrap();

        let delete_slot_req = http::Request::builder()
            .method("DELETE")
            .uri(&format!("/api/projects/{}/slots/{}", project_id, slot_id))
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();
        let delete_slot_res = app.clone().oneshot(delete_slot_req).await.unwrap();
        assert_eq!(delete_slot_res.status(), http::StatusCode::NO_CONTENT);

        let node_after = boardtask::app::db::nodes::find_by_id(&pool, &node_id).await.unwrap().unwrap();
        assert!(node_after.slot_id.is_none());
    }

    #[tokio::test]
    async fn patch_node_invalid_slot_id_returns_400() {
        let (cookie, project_id, pool, app) = setup_user_and_project("patchinvslot@example.com", "Password123").await;

        let node_id = ulid::Ulid::new().to_string();
        let node = db::nodes::NewNode {
            id: node_id.clone(),
            project_id: project_id.clone(),
            node_type_id: TASK_NODE_TYPE_ID.to_string(),
            status_id: DEFAULT_STATUS_ID.to_string(),
            title: "Node".to_string(),
            description: None,
            estimated_minutes: None,
            slot_id: None,
            parent_id: None,
        };
        boardtask::app::db::nodes::insert(&pool, &node).await.unwrap();

        let patch_body = serde_json::json!({ "slot_id": "invalid-slot-id" });
        let request = http::Request::builder()
            .method("PATCH")
            .uri(&format!("/api/projects/{}/nodes/{}", project_id, node_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(patch_body.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["error"], "Invalid slot_id");
    }

    #[tokio::test]
    async fn patch_node_set_parent_id_then_clear() {
        let (cookie, project_id, _pool, app) = setup_user_and_project("patchparent@example.com", "Password123").await;

        let post_group_req = http::Request::builder()
            .method("POST")
            .uri(&format!("/api/projects/{}/nodes", project_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(
                serde_json::json!({ "node_type_id": TASK_NODE_TYPE_ID, "title": "Group" }).to_string(),
            ))
            .unwrap();
        let post_group_res = app.clone().oneshot(post_group_req).await.unwrap();
        assert_eq!(post_group_res.status(), http::StatusCode::CREATED);
        let group_body: serde_json::Value =
            serde_json::from_slice(&post_group_res.into_body().collect().await.unwrap().to_bytes()).unwrap();
        let group_id = group_body["id"].as_str().unwrap();

        let post_child_req = http::Request::builder()
            .method("POST")
            .uri(&format!("/api/projects/{}/nodes", project_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(
                serde_json::json!({ "node_type_id": TASK_NODE_TYPE_ID, "title": "Child" }).to_string(),
            ))
            .unwrap();
        let post_child_res = app.clone().oneshot(post_child_req).await.unwrap();
        assert_eq!(post_child_res.status(), http::StatusCode::CREATED);
        let child_body: serde_json::Value =
            serde_json::from_slice(&post_child_res.into_body().collect().await.unwrap().to_bytes()).unwrap();
        let child_id = child_body["id"].as_str().unwrap();

        let patch_body = serde_json::json!({ "parent_id": group_id });
        let patch_request = http::Request::builder()
            .method("PATCH")
            .uri(&format!("/api/projects/{}/nodes/{}", project_id, child_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(patch_body.to_string()))
            .unwrap();

        let patch_response = app.clone().oneshot(patch_request).await.unwrap();
        assert_eq!(patch_response.status(), http::StatusCode::OK);

        let patch_res_bytes = patch_response.into_body().collect().await.unwrap().to_bytes();
        let patch_res: serde_json::Value = serde_json::from_slice(&patch_res_bytes).unwrap();
        assert_eq!(patch_res["parent_id"], group_id);

        let get_request = http::Request::builder()
            .method("GET")
            .uri(&format!("/api/projects/{}/graph", project_id))
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();
        let get_response = app.clone().oneshot(get_request).await.unwrap();
        assert_eq!(get_response.status(), http::StatusCode::OK);
        let get_bytes = get_response.into_body().collect().await.unwrap().to_bytes();
        let graph: serde_json::Value = serde_json::from_slice(&get_bytes).unwrap();
        let nodes = graph["nodes"].as_array().unwrap();
        let updated_node = nodes.iter().find(|n| n["id"] == child_id).unwrap();
        assert_eq!(updated_node["parent_id"], group_id);

        let clear_body = serde_json::json!({ "parent_id": null });
        let clear_request = http::Request::builder()
            .method("PATCH")
            .uri(&format!("/api/projects/{}/nodes/{}", project_id, child_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(clear_body.to_string()))
            .unwrap();

        let clear_response = app.clone().oneshot(clear_request).await.unwrap();
        assert_eq!(clear_response.status(), http::StatusCode::OK);

        let clear_res_bytes = clear_response.into_body().collect().await.unwrap().to_bytes();
        let clear_res: serde_json::Value = serde_json::from_slice(&clear_res_bytes).unwrap();
        assert!(clear_res["parent_id"].is_null());

        let get_request2 = http::Request::builder()
            .method("GET")
            .uri(&format!("/api/projects/{}/graph", project_id))
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();
        let get_response2 = app.clone().oneshot(get_request2).await.unwrap();
        let get_bytes2 = get_response2.into_body().collect().await.unwrap().to_bytes();
        let graph2: serde_json::Value = serde_json::from_slice(&get_bytes2).unwrap();
        let nodes2 = graph2["nodes"].as_array().unwrap();
        let cleared_node = nodes2.iter().find(|n| n["id"] == child_id).unwrap();
        assert!(cleared_node["parent_id"].is_null());
    }

    #[tokio::test]
    async fn patch_node_invalid_parent_id_returns_400() {
        let (cookie, project_id, pool, app) = setup_user_and_project("patchinvparent@example.com", "Password123").await;

        let node_id = ulid::Ulid::new().to_string();
        let node = db::nodes::NewNode {
            id: node_id.clone(),
            project_id: project_id.clone(),
            node_type_id: TASK_NODE_TYPE_ID.to_string(),
            status_id: DEFAULT_STATUS_ID.to_string(),
            title: "Node".to_string(),
            description: None,
            estimated_minutes: None,
            slot_id: None,
            parent_id: None,
        };
        boardtask::app::db::nodes::insert(&pool, &node).await.unwrap();

        let patch_body = serde_json::json!({ "parent_id": "nonexistent-id" });
        let request = http::Request::builder()
            .method("PATCH")
            .uri(&format!("/api/projects/{}/nodes/{}", project_id, node_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(patch_body.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["error"], "Invalid parent_id");
    }

    #[tokio::test]
    async fn patch_node_parent_id_from_other_project_returns_400() {
        let (cookie, project_id, pool, app) = setup_user_and_project("patchotherparent@example.com", "Password123").await;
        let user_id = user_id_from_cookie(&pool, &cookie).await;
        let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
        let org_id = user.organization_id.clone();

        let project_node_id = ulid::Ulid::new().to_string();
        let project_node = db::nodes::NewNode {
            id: project_node_id.clone(),
            project_id: project_id.clone(),
            node_type_id: TASK_NODE_TYPE_ID.to_string(),
            status_id: DEFAULT_STATUS_ID.to_string(),
            title: "Project Node".to_string(),
            description: None,
            estimated_minutes: None,
            slot_id: None,
            parent_id: None,
        };
        boardtask::app::db::nodes::insert(&pool, &project_node).await.unwrap();

        let other_project_id = ulid::Ulid::new().to_string();
        let other_project = db::NewProject {
            id: other_project_id.clone(),
            title: "Other Project".to_string(),
            user_id: user_id.clone(),
            organization_id: org_id.clone(),
        };
        boardtask::app::db::projects::insert(&pool, &other_project).await.unwrap();

        let other_node_id = ulid::Ulid::new().to_string();
        let other_node = db::nodes::NewNode {
            id: other_node_id.clone(),
            project_id: other_project_id.clone(),
            node_type_id: TASK_NODE_TYPE_ID.to_string(),
            status_id: DEFAULT_STATUS_ID.to_string(),
            title: "Other Node".to_string(),
            description: None,
            estimated_minutes: None,
            slot_id: None,
            parent_id: None,
        };
        boardtask::app::db::nodes::insert(&pool, &other_node).await.unwrap();

        let patch_body = serde_json::json!({ "parent_id": other_node_id });
        let request = http::Request::builder()
            .method("PATCH")
            .uri(&format!("/api/projects/{}/nodes/{}", project_id, project_node_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(patch_body.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["error"], "Invalid parent_id");
    }
}

mod edges {
    use super::*;

    #[tokio::test]
    async fn post_edge_requires_authentication() {
    let pool = test_pool().await;
    let app = test_router(pool);

    let request_body = serde_json::json!({
        "parent_id": "parent-id",
        "child_id": "child-id"
    });

    let request = http::Request::builder()
        .method("POST")
        .uri("/api/projects/123/edges")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Unauthorized");
}

#[tokio::test]
async fn post_edge_succeeds() {
    let (cookie, project_id, pool, app) = setup_user_and_project("edge@example.com", "Password123").await;

    // Create two nodes in the project
    let parent_node_id = ulid::Ulid::new().to_string();
    let child_node_id = ulid::Ulid::new().to_string();

    let parent_node = db::nodes::NewNode {
        id: parent_node_id.clone(),
        project_id: project_id.clone(),
        node_type_id: TASK_NODE_TYPE_ID.to_string(),
        status_id: DEFAULT_STATUS_ID.to_string(),
        title: "Parent Node".to_string(),
        description: None,
        estimated_minutes: None,
        slot_id: None,
        parent_id: None,
    };
    let child_node = db::nodes::NewNode {
        id: child_node_id.clone(),
        project_id: project_id.clone(),
        node_type_id: TASK_NODE_TYPE_ID.to_string(),
        status_id: DEFAULT_STATUS_ID.to_string(),
        title: "Child Node".to_string(),
        description: None,
        estimated_minutes: None,
        slot_id: None,
        parent_id: None,
    };

    boardtask::app::db::nodes::insert(&pool, &parent_node).await.unwrap();
    boardtask::app::db::nodes::insert(&pool, &child_node).await.unwrap();

    let request_body = serde_json::json!({
        "parent_id": parent_node_id,
        "child_id": child_node_id
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/edges", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::CREATED);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["parent_id"], parent_node_id);
    assert_eq!(body["child_id"], child_node_id);
    assert!(body["created_at"].is_number());
}

#[tokio::test]
async fn post_edge_rejects_self_referential() {
    let (cookie, project_id, pool, app) = setup_user_and_project("selfedge@example.com", "Password123").await;

    let node_id = ulid::Ulid::new().to_string();
    let node = db::nodes::NewNode {
        id: node_id.clone(),
        project_id: project_id.clone(),
        node_type_id: TASK_NODE_TYPE_ID.to_string(),
        status_id: DEFAULT_STATUS_ID.to_string(),
        title: "Single Node".to_string(),
        description: None,
        estimated_minutes: None,
        slot_id: None,
        parent_id: None,
    };
    boardtask::app::db::nodes::insert(&pool, &node).await.unwrap();

    let request_body = serde_json::json!({
        "parent_id": node_id,
        "child_id": node_id
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/edges", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body["error"].as_str().unwrap().contains("node to itself"));
}

#[tokio::test]
async fn post_edge_duplicate_returns_conflict_or_error() {
    let (cookie, project_id, pool, app) = setup_user_and_project("dupedge@example.com", "Password123").await;

    let parent_node_id = ulid::Ulid::new().to_string();
    let child_node_id = ulid::Ulid::new().to_string();
    let parent_node = db::nodes::NewNode {
        id: parent_node_id.clone(),
        project_id: project_id.clone(),
        node_type_id: TASK_NODE_TYPE_ID.to_string(),
        status_id: DEFAULT_STATUS_ID.to_string(),
        title: "Parent".to_string(),
        description: None,
        estimated_minutes: None,
        slot_id: None,
        parent_id: None,
    };
    let child_node = db::nodes::NewNode {
        id: child_node_id.clone(),
        project_id: project_id.clone(),
        node_type_id: TASK_NODE_TYPE_ID.to_string(),
        status_id: DEFAULT_STATUS_ID.to_string(),
        title: "Child".to_string(),
        description: None,
        estimated_minutes: None,
        slot_id: None,
        parent_id: None,
    };
    boardtask::app::db::nodes::insert(&pool, &parent_node).await.unwrap();
    boardtask::app::db::nodes::insert(&pool, &child_node).await.unwrap();

    let request_body = serde_json::json!({
        "parent_id": parent_node_id,
        "child_id": child_node_id
    });

    let request1 = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/edges", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();
    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), http::StatusCode::CREATED);

    let request2 = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/edges", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();
    let response2 = app.oneshot(request2).await.unwrap();
    assert!(!response2.status().is_success(), "duplicate edge should not succeed");
    assert_eq!(response2.status(), http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn post_edge_404_when_node_not_in_project() {
    let (cookie, project_id, pool, app) = setup_user_and_project("edge404@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
    let org_id = user.organization_id.clone();

    // Create a node in the project
    let project_node_id = ulid::Ulid::new().to_string();
    let project_node = db::nodes::NewNode {
        id: project_node_id.clone(),
        project_id: project_id.clone(),
        node_type_id: TASK_NODE_TYPE_ID.to_string(),
        status_id: DEFAULT_STATUS_ID.to_string(),
        title: "Project Node".to_string(),
        description: None,
        estimated_minutes: None,
        slot_id: None,
        parent_id: None,
    };
    boardtask::app::db::nodes::insert(&pool, &project_node).await.unwrap();

    // Create another project and node
    let other_project_id = ulid::Ulid::new().to_string();
    let other_project = db::NewProject {
        id: other_project_id.clone(),
        title: "Other Project".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
    };
    boardtask::app::db::projects::insert(&pool, &other_project).await.unwrap();

    let other_node_id = ulid::Ulid::new().to_string();
    let other_node = db::nodes::NewNode {
        id: other_node_id.clone(),
        project_id: other_project_id.clone(),
        node_type_id: TASK_NODE_TYPE_ID.to_string(),
        status_id: DEFAULT_STATUS_ID.to_string(),
        title: "Other Node".to_string(),
        description: None,
        estimated_minutes: None,
        slot_id: None,
        parent_id: None,
    };
    boardtask::app::db::nodes::insert(&pool, &other_node).await.unwrap();

    // Try to create edge between nodes from different projects
    let request_body = serde_json::json!({
        "parent_id": project_node_id,
        "child_id": other_node_id
    });

    let request = http::Request::builder()
        .method("POST")
        .uri(&format!("/api/projects/{}/edges", project_id))
        .header("content-type", "application/json")
        .header("cookie", &cookie)
        .body(axum::body::Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::NOT_FOUND);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Child node not found");
    }
}

mod get_graph {
    use super::*;

    #[tokio::test]
    async fn get_graph_requires_authentication() {
        let pool = test_pool().await;
        let app = test_router(pool);

        let request = http::Request::builder()
            .method("GET")
            .uri("/api/projects/123/graph")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["error"], "Unauthorized");

        let slots_request = http::Request::builder()
            .method("GET")
            .uri("/api/projects/123/slots")
            .body(axum::body::Body::empty())
            .unwrap();
        let slots_response = app.oneshot(slots_request).await.unwrap();
        assert_eq!(slots_response.status(), http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn get_graph_succeeds() {
        let (cookie, project_id, pool, app) = setup_user_and_project("getgraph@example.com", "Password123").await;

        // GET slots (empty), POST slot, GET slots, POST duplicate name → 400
        let get_slots_req = http::Request::builder()
            .method("GET")
            .uri(&format!("/api/projects/{}/slots", project_id))
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();
        let get_slots_res = app.clone().oneshot(get_slots_req).await.unwrap();
        assert_eq!(get_slots_res.status(), http::StatusCode::OK);
        let slots_body: serde_json::Value = serde_json::from_slice(&get_slots_res.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert!(slots_body["slots"].as_array().unwrap().is_empty());

        let post_slot_body = serde_json::json!({ "name": "Front-end Developer 1", "sort_order": 0 });
        let post_slot_req = http::Request::builder()
            .method("POST")
            .uri(&format!("/api/projects/{}/slots", project_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(post_slot_body.to_string()))
            .unwrap();
        let post_slot_res = app.clone().oneshot(post_slot_req).await.unwrap();
        assert_eq!(post_slot_res.status(), http::StatusCode::CREATED);
        let created_slot: serde_json::Value = serde_json::from_slice(&post_slot_res.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(created_slot["name"], "Front-end Developer 1");
        assert_eq!(created_slot["project_id"], project_id);

        let get_slots_req2 = http::Request::builder()
            .method("GET")
            .uri(&format!("/api/projects/{}/slots", project_id))
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();
        let get_slots_res2 = app.clone().oneshot(get_slots_req2).await.unwrap();
        let slots_body2: serde_json::Value = serde_json::from_slice(&get_slots_res2.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(slots_body2["slots"].as_array().unwrap().len(), 1);

        let post_dup_req = http::Request::builder()
            .method("POST")
            .uri(&format!("/api/projects/{}/slots", project_id))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(axum::body::Body::from(serde_json::json!({ "name": "Front-end Developer 1" }).to_string()))
            .unwrap();
        let post_dup_res = app.clone().oneshot(post_dup_req).await.unwrap();
        assert_eq!(post_dup_res.status(), http::StatusCode::BAD_REQUEST);

        // Create nodes in the project
        let node1_id = ulid::Ulid::new().to_string();
        let node1 = db::nodes::NewNode {
            id: node1_id.clone(),
            project_id: project_id.clone(),
            node_type_id: TASK_NODE_TYPE_ID.to_string(),
            status_id: DEFAULT_STATUS_ID.to_string(),
            title: "Node 1".to_string(),
            description: Some("First node".to_string()),
            estimated_minutes: None,
            slot_id: None,
            parent_id: None,
        };
        boardtask::app::db::nodes::insert(&pool, &node1).await.unwrap();

        let node2_id = ulid::Ulid::new().to_string();
        let node2 = db::nodes::NewNode {
            id: node2_id.clone(),
            project_id: project_id.clone(),
            node_type_id: TASK_NODE_TYPE_ID.to_string(),
            status_id: DEFAULT_STATUS_ID.to_string(),
            title: "Node 2".to_string(),
            description: None,
            estimated_minutes: None,
            slot_id: None,
            parent_id: None,
        };
        boardtask::app::db::nodes::insert(&pool, &node2).await.unwrap();

        // Create an edge
        let edge = db::node_edges::NewNodeEdge {
            parent_id: node1_id.clone(),
            child_id: node2_id.clone(),
        };
        boardtask::app::db::node_edges::insert(&pool, &edge).await.unwrap();

        // Get the graph
        let request = http::Request::builder()
            .method("GET")
            .uri(&format!("/api/projects/{}/graph", project_id))
            .header("cookie", &cookie)
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::OK);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        // Check nodes
        assert!(body["nodes"].is_array());
        assert_eq!(body["nodes"].as_array().unwrap().len(), 2);

        let node_titles: std::collections::HashSet<&str> = body["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["title"].as_str().unwrap())
            .collect();
        assert_eq!(node_titles, std::collections::HashSet::from(["Node 1", "Node 2"]));

        // Check edges
        assert!(body["edges"].is_array());
        assert_eq!(body["edges"].as_array().unwrap().len(), 1);

        let edge = &body["edges"][0];
        assert_eq!(edge["parent_id"], node1_id);
        assert_eq!(edge["child_id"], node2_id);
    }
}