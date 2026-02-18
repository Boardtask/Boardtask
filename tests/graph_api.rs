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
    let pool = test_pool().await;
    let app = test_router(pool.clone());
    ensure_graph_seeds(&pool).await;

    let cookie = authenticated_cookie(&pool, &app, "node@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
    let org_id = user.organization_id.clone();

    // Create a project for the user
    let project_id = ulid::Ulid::new().to_string();
    let project = db::NewProject {
        id: project_id.clone(),
        title: "Test Project".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
    };
    boardtask::app::db::projects::insert(&pool, &project).await.unwrap();

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

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::NOT_FOUND);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"], "Project not found");
}

#[tokio::test]
async fn post_node_404_for_project_owned_by_other_user() {
    let pool = test_pool().await;
    let app = test_router(pool.clone());
    ensure_graph_seeds(&pool).await;

    // Create user A with a project
    let cookie_a = authenticated_cookie(&pool, &app, "usera@example.com", "Password123").await;
    let user_a_id = user_id_from_cookie(&pool, &cookie_a).await;
    let user_a = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_a_id).unwrap()).await.unwrap().unwrap();
    let org_a_id = user_a.organization_id.clone();

    let project_id = ulid::Ulid::new().to_string();
    let project = db::NewProject {
        id: project_id.clone(),
        title: "User A Project".to_string(),
        user_id: user_a_id,
        organization_id: org_a_id.clone(),
    };
    boardtask::app::db::projects::insert(&pool, &project).await.unwrap();

    // Create user B
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
    let pool = test_pool().await;
    let app = test_router(pool.clone());
    ensure_graph_seeds(&pool).await;

    let cookie = authenticated_cookie(&pool, &app, "invalidtype@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
    let org_id = user.organization_id.clone();

    // Create a project for the user
    let project_id = ulid::Ulid::new().to_string();
    let project = db::NewProject {
        id: project_id.clone(),
        title: "Test Project".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
    };
    boardtask::app::db::projects::insert(&pool, &project).await.unwrap();

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
    let pool = test_pool().await;
    let app = test_router(pool.clone());
    ensure_graph_seeds(&pool).await;

    let cookie = authenticated_cookie(&pool, &app, "invalidstatus@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
    let org_id = user.organization_id.clone();

    let project_id = ulid::Ulid::new().to_string();
    let project = db::NewProject {
        id: project_id.clone(),
        title: "Test Project".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
    };
    boardtask::app::db::projects::insert(&pool, &project).await.unwrap();

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
async fn post_node_with_status_then_patch_and_get_graph() {
    let pool = test_pool().await;
    let app = test_router(pool.clone());
    ensure_graph_seeds(&pool).await;

    let cookie = authenticated_cookie(&pool, &app, "statusflow@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
    let org_id = user.organization_id.clone();

    let project_id = ulid::Ulid::new().to_string();
    let project = db::NewProject {
        id: project_id.clone(),
        title: "Status Test Project".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
    };
    boardtask::app::db::projects::insert(&pool, &project).await.unwrap();

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
        let pool = test_pool().await;
        let app = test_router(pool.clone());
        ensure_graph_seeds(&pool).await;

        let cookie = authenticated_cookie(&pool, &app, "patchclear@example.com", "Password123").await;
        let user_id = user_id_from_cookie(&pool, &cookie).await;
        let user = boardtask::app::db::users::find_by_id(
            &pool,
            &boardtask::app::domain::UserId::from_string(&user_id).unwrap(),
        )
        .await
        .unwrap()
        .unwrap();
        let org_id = user.organization_id.clone();

        let project_id = ulid::Ulid::new().to_string();
        let project = db::NewProject {
            id: project_id.clone(),
            title: "Patch Test Project".to_string(),
            user_id: user_id.clone(),
            organization_id: org_id.clone(),
        };
        boardtask::app::db::projects::insert(&pool, &project).await.unwrap();

        let node_id = ulid::Ulid::new().to_string();
        let node = db::nodes::NewNode {
            id: node_id.clone(),
            project_id: project_id.clone(),
            node_type_id: TASK_NODE_TYPE_ID.to_string(),
            status_id: DEFAULT_STATUS_ID.to_string(),
            title: "Node With Estimate".to_string(),
            description: None,
            estimated_minutes: Some(30),
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
    let pool = test_pool().await;
    let app = test_router(pool.clone());
    ensure_graph_seeds(&pool).await;

    let cookie = authenticated_cookie(&pool, &app, "edge@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
    let org_id = user.organization_id.clone();

    // Create a project for the user
    let project_id = ulid::Ulid::new().to_string();
    let project = db::NewProject {
        id: project_id.clone(),
        title: "Test Project".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
    };
    boardtask::app::db::projects::insert(&pool, &project).await.unwrap();

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
    };
    let child_node = db::nodes::NewNode {
        id: child_node_id.clone(),
        project_id: project_id.clone(),
        node_type_id: TASK_NODE_TYPE_ID.to_string(),
        status_id: DEFAULT_STATUS_ID.to_string(),
        title: "Child Node".to_string(),
        description: None,
        estimated_minutes: None,
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
async fn post_edge_404_when_node_not_in_project() {
    let pool = test_pool().await;
    let app = test_router(pool.clone());
    ensure_graph_seeds(&pool).await;

    let cookie = authenticated_cookie(&pool, &app, "edge404@example.com", "Password123").await;
    let user_id = user_id_from_cookie(&pool, &cookie).await;
    let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
    let org_id = user.organization_id.clone();

    // Create a project for the user
    let project_id = ulid::Ulid::new().to_string();
    let project = db::NewProject {
        id: project_id.clone(),
        title: "Test Project".to_string(),
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
    };
    boardtask::app::db::projects::insert(&pool, &project).await.unwrap();

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

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["error"], "Unauthorized");
    }

    #[tokio::test]
    async fn get_graph_succeeds() {
        let pool = test_pool().await;
        let app = test_router(pool.clone());
        ensure_graph_seeds(&pool).await;

        let cookie = authenticated_cookie(&pool, &app, "getgraph@example.com", "Password123").await;
        let user_id = user_id_from_cookie(&pool, &cookie).await;
        let user = boardtask::app::db::users::find_by_id(&pool, &boardtask::app::domain::UserId::from_string(&user_id).unwrap()).await.unwrap().unwrap();
        let org_id = user.organization_id.clone();

        // Create a project for the user
        let project_id = ulid::Ulid::new().to_string();
        let project = db::NewProject {
            id: project_id.clone(),
            title: "Test Project".to_string(),
            user_id: user_id.clone(),
            organization_id: org_id.clone(),
        };
        boardtask::app::db::projects::insert(&pool, &project).await.unwrap();

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