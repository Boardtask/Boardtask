mod auth {
    mod common {
    use boardtask::create_router;
    use sqlx::SqlitePool;

    pub async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    pub fn test_router(pool: SqlitePool) -> axum::Router {
        let state = boardtask::app::AppState {
            db: pool,
            mail: std::sync::Arc::new(boardtask::app::mail::ConsoleMailer),
        };
        create_router(state)
    }
    }

    mod signup {
        use super::common::*;
    use axum::body::Body;
    use http::StatusCode;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn signup_form_body(email: &str, password: &str, confirm_password: &str) -> String {
        format!(
            "email={}&password={}&confirm_password={}",
            urlencoding::encode(email),
            urlencoding::encode(password),
            urlencoding::encode(confirm_password)
        )
    }

    #[tokio::test]
    async fn creates_user_and_redirects() {
        let pool = test_pool().await;
        let app = test_router(pool);

        let body = signup_form_body("test@example.com", "Password123", "Password123");
        let request = http::Request::builder()
            .method("POST")
            .uri("/signup")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get("location").map(|v| v.to_str().unwrap()),
            Some("/app")
        );
        assert!(
            response.headers().get("set-cookie").is_some(),
            "Expected Set-Cookie header on successful signup"
        );
    }

    #[tokio::test]
    async fn duplicate_email_returns_error() {
        let pool = test_pool().await;
        let app = test_router(pool);

        let body = signup_form_body("dup@example.com", "Password123", "Password123");

        // First signup succeeds
        let request1 = http::Request::builder()
            .method("POST")
            .uri("/signup")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body.clone()))
            .unwrap();
        let response1 = app.clone().oneshot(request1).await.unwrap();
        assert_eq!(response1.status(), StatusCode::SEE_OTHER);

        // Second signup with same email fails
        let request2 = http::Request::builder()
            .method("POST")
            .uri("/signup")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap();
        let response2 = app.oneshot(request2).await.unwrap();

        assert_eq!(response2.status(), StatusCode::OK);
        let body_bytes = response2.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("Unable to create account"),
            "Expected generic signup error, got: {}",
            body_str
        );
    }
    }

    mod login {
        use super::common::*;
    use axum::body::Body;
    use http::StatusCode;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn login_form_body(email: &str, password: &str) -> String {
        format!(
            "email={}&password={}",
            urlencoding::encode(email),
            urlencoding::encode(password)
        )
    }

    fn signup_form_body(email: &str, password: &str, confirm_password: &str) -> String {
        format!(
            "email={}&password={}&confirm_password={}",
            urlencoding::encode(email),
            urlencoding::encode(password),
            urlencoding::encode(confirm_password)
        )
    }

    #[tokio::test]
    async fn valid_credentials_redirect_to_dashboard() {
        let pool = test_pool().await;
        let app = test_router(pool);

        // Sign up first
        let signup_body =
            signup_form_body("login@example.com", "Password123", "Password123");
        let signup_request = http::Request::builder()
            .method("POST")
            .uri("/signup")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(signup_body))
            .unwrap();
        let signup_response = app.clone().oneshot(signup_request).await.unwrap();
        assert_eq!(signup_response.status(), StatusCode::SEE_OTHER);

        // Then log in
        let login_body = login_form_body("login@example.com", "Password123");
        let login_request = http::Request::builder()
            .method("POST")
            .uri("/login")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(login_body))
            .unwrap();
        let login_response = app.oneshot(login_request).await.unwrap();

        assert_eq!(login_response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            login_response.headers().get("location").map(|v| v.to_str().unwrap()),
            Some("/app")
        );
        assert!(
            login_response.headers().get("set-cookie").is_some(),
            "Expected Set-Cookie header on successful login"
        );
    }

    #[tokio::test]
    async fn invalid_credentials_returns_error() {
        let pool = test_pool().await;
        let app = test_router(pool);

        let body = login_form_body("nonexistent@example.com", "Wrongpassword1");
        let request = http::Request::builder()
            .method("POST")
            .uri("/login")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("Invalid") || body_str.contains("invalid"),
            "Expected error message, got: {}",
            body_str
        );
    }

    #[tokio::test]
    async fn signup_then_login_succeeds() {
        let pool = test_pool().await;
        let app = test_router(pool);

        // Sign up
        let signup_body =
            signup_form_body("flow@example.com", "Secretpass99", "Secretpass99");
        let signup_request = http::Request::builder()
            .method("POST")
            .uri("/signup")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(signup_body))
            .unwrap();
        let signup_response = app.clone().oneshot(signup_request).await.unwrap();
        assert_eq!(
            signup_response.status(),
            StatusCode::SEE_OTHER,
            "Signup should succeed"
        );

        // Login with same credentials
        let login_body = login_form_body("flow@example.com", "Secretpass99");
        let login_request = http::Request::builder()
            .method("POST")
            .uri("/login")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(login_body))
            .unwrap();
        let login_response = app.oneshot(login_request).await.unwrap();

        assert_eq!(
            login_response.status(),
            StatusCode::SEE_OTHER,
            "Login after signup should succeed"
        );
        assert_eq!(
            login_response.headers().get("location").map(|v| v.to_str().unwrap()),
            Some("/app")
        );
    }
    }

    mod logout {
        use super::common::*;
        use axum::body::Body;
        use boardtask::app::db;
        use http::StatusCode;
        use tower::ServiceExt;

        fn signup_form_body(email: &str, password: &str, confirm_password: &str) -> String {
            format!(
                "email={}&password={}&confirm_password={}",
                urlencoding::encode(email),
                urlencoding::encode(password),
                urlencoding::encode(confirm_password)
            )
        }

        fn extract_session_id_from_cookie(set_cookie_header: &str) -> Option<&str> {
            // Parse "session_id=abc123; Path=/; HttpOnly; SameSite=Lax"
            set_cookie_header.split(';').next()?.strip_prefix("session_id=")
        }

        /// Asserts that among all Set-Cookie headers, one is a removal cookie (Max-Age=0) for session_id.
        fn assert_removal_cookie_in_response<B>(response: &http::Response<B>) {
            let cookies: Vec<_> = response
                .headers()
                .get_all("set-cookie")
                .iter()
                .filter_map(|v| v.to_str().ok())
                .collect();
            assert!(
                cookies.iter().any(|c| {
                    let c_lower = c.to_lowercase();
                    c.contains("session_id=") && c_lower.contains("max-age=0")
                }),
                "Expected removal cookie for session_id with Max-Age=0 among Set-Cookie headers, got: {:?}",
                cookies
            );
        }

        #[tokio::test]
        async fn logout_clears_session_and_redirects() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());

            // Sign up to get a session
            let signup_body = signup_form_body("logout@example.com", "Password123", "Password123");
            let signup_request = http::Request::builder()
                .method("POST")
                .uri("/signup")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(signup_body))
                .unwrap();
            let signup_response = app.clone().oneshot(signup_request).await.unwrap();
            assert_eq!(signup_response.status(), StatusCode::SEE_OTHER);

            // Extract session cookie
            let set_cookie = signup_response.headers()
                .get("set-cookie")
                .unwrap()
                .to_str()
                .unwrap();
            let session_id = extract_session_id_from_cookie(set_cookie).unwrap();

            // Logout with session cookie
            let logout_request = http::Request::builder()
                .method("POST")
                .uri("/logout")
                .header("cookie", format!("session_id={}", session_id))
                .body(Body::empty())
                .unwrap();
            let logout_response = app.oneshot(logout_request).await.unwrap();

            assert_eq!(logout_response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                logout_response.headers().get("location").map(|v| v.to_str().unwrap()),
                Some("/")
            );
            assert_removal_cookie_in_response(&logout_response);

            // Verify session was deleted from DB (complete logout contract)
            assert!(
                db::sessions::find_valid(&pool, session_id).await.unwrap().is_none(),
                "Session should be removed from database on logout"
            );
        }

        #[tokio::test]
        async fn logout_without_cookie_sends_removal_cookie() {
            // Logout without a session cookie must still send Set-Cookie to clear any stale
            // session_id in the browser. jar.remove() would not add to delta when there's
            // no original cookie - this test catches that bug.
            let pool = test_pool().await;
            let app = test_router(pool);

            let request = http::Request::builder()
                .method("POST")
                .uri("/logout")
                .body(Body::empty())
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                response.headers().get("location").map(|v| v.to_str().unwrap()),
                Some("/")
            );
            assert_removal_cookie_in_response(&response);
        }
    }

    mod dashboard {
        use super::common::*;
        use axum::body::Body;
        use http::StatusCode;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        #[tokio::test]
        async fn dashboard_renders_with_logout_form() {
            let pool = test_pool().await;
            let app = test_router(pool);

            let request = http::Request::builder()
                .method("GET")
                .uri("/app")
                .body(Body::empty())
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8_lossy(&body_bytes);
            assert!(
                body_str.contains("Log Out") || body_str.contains("action=\"/logout\""),
                "Expected logout form or button in dashboard, got: {}",
                body_str
            );
        }
    }
}
