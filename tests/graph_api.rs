use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

use crate::common::*;
use boardtask::app::db;

const TASK_NODE_TYPE_ID: &str = "01JNODETYPE00000000TASK000";

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
        "description": "A test node"
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
        title: "Parent Node".to_string(),
        description: None,
    };
    let child_node = db::nodes::NewNode {
        id: child_node_id.clone(),
        project_id: project_id.clone(),
        node_type_id: TASK_NODE_TYPE_ID.to_string(),
        title: "Child Node".to_string(),
        description: None,
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
        title: "Project Node".to_string(),
        description: None,
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
        title: "Other Node".to_string(),
        description: None,
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
            title: "Node 1".to_string(),
            description: Some("First node".to_string()),
            };
        boardtask::app::db::nodes::insert(&pool, &node1).await.unwrap();

        let node2_id = ulid::Ulid::new().to_string();
        let node2 = db::nodes::NewNode {
            id: node2_id.clone(),
            project_id: project_id.clone(),
            node_type_id: TASK_NODE_TYPE_ID.to_string(),
            title: "Node 2".to_string(),
            description: None,
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