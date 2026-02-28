mod common;

mod auth {
    mod signup {
        use crate::common::*;
        use axum::body::Body;
        use http::StatusCode;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

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
        assert!(
            response.headers().get("location").map(|v| v.to_str().unwrap()).unwrap().starts_with("/check-email"),
            "Expected redirect to /check-email on successful signup"
        );
        assert!(
            response.headers().get("set-cookie").is_none(),
            "Should NOT have Set-Cookie header on successful signup (no session created)"
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
        use crate::common::*;
        use axum::body::Body;
        use http::StatusCode;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        #[tokio::test]
    async fn valid_credentials_redirect_to_dashboard() {
        let pool = test_pool().await;
        let app = test_router(pool.clone());

        // Create a verified user directly (bypassing signup flow for this test)
        create_verified_user(&pool, "login@example.com", "Password123").await;

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
    async fn login_unverified_returns_error() {
        let pool = test_pool().await;
        let app = test_router(pool);

        // Sign up (creates unverified user)
        let signup_body = signup_form_body("unverified@example.com", "Password123", "Password123");
        let signup_request = http::Request::builder()
            .method("POST")
            .uri("/signup")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(signup_body))
            .unwrap();
        let signup_response = app.clone().oneshot(signup_request).await.unwrap();
        assert_eq!(signup_response.status(), StatusCode::SEE_OTHER);

        // Try to log in (should fail)
        let login_body = login_form_body("unverified@example.com", "Password123");
        let login_request = http::Request::builder()
            .method("POST")
            .uri("/login")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(login_body))
            .unwrap();
        let login_response = app.oneshot(login_request).await.unwrap();

        assert_eq!(login_response.status(), StatusCode::OK);
        assert!(
            login_response.headers().get("set-cookie").is_none(),
            "Should NOT have Set-Cookie header when login fails"
        );

        let body_bytes = login_response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("verify your email"),
            "Expected email verification error, got: {}",
            body_str
        );
    }

    #[tokio::test]
    async fn signup_then_verify_then_login_succeeds() {
        let pool = test_pool().await;
        let app = test_router(pool.clone());

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

        // Get the verification token from the database
        let token: String = sqlx::query_scalar("SELECT token FROM email_verification_tokens ORDER BY created_at DESC LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        // Verify the email
        let verify_request = http::Request::builder()
            .method("GET")
            .uri(&format!("/verify-email?token={}", token))
            .body(Body::empty())
            .unwrap();
        let verify_response = app.clone().oneshot(verify_request).await.unwrap();
        assert_eq!(
            verify_response.status(),
            StatusCode::SEE_OTHER,
            "Email verification should succeed"
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
            "Login after verification should succeed"
        );
        assert_eq!(
            login_response.headers().get("location").map(|v| v.to_str().unwrap()),
            Some("/app")
        );
    }
    }

    mod logout {
        use crate::common::*;
        use axum::body::Body;
        use boardtask::app::db;
        use http::StatusCode;
        use tower::ServiceExt;

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

            // Sign up and verify to get a session
            let signup_body = signup_form_body("logout@example.com", "Password123", "Password123");
            let signup_request = http::Request::builder()
                .method("POST")
                .uri("/signup")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(signup_body))
                .unwrap();
            let signup_response = app.clone().oneshot(signup_request).await.unwrap();
            assert_eq!(signup_response.status(), StatusCode::SEE_OTHER);

            // Verify the email
            let token: String = sqlx::query_scalar("SELECT token FROM email_verification_tokens ORDER BY created_at DESC LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
            let verify_request = http::Request::builder()
                .method("GET")
                .uri(&format!("/verify-email?token={}", token))
                .body(Body::empty())
                .unwrap();
            let verify_response = app.clone().oneshot(verify_request).await.unwrap();
            assert_eq!(verify_response.status(), StatusCode::SEE_OTHER);

            // Login to get session cookie
            let login_body = login_form_body("logout@example.com", "Password123");
            let login_request = http::Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(login_body))
                .unwrap();
            let login_response = app.clone().oneshot(login_request).await.unwrap();
            assert_eq!(login_response.status(), StatusCode::SEE_OTHER);

            // Extract session cookie from login response
            let set_cookie = login_response.headers()
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
        use crate::common::*;
        use axum::body::Body;
        use http::StatusCode;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        #[tokio::test]
        async fn dashboard_requires_authentication() {
            let pool = test_pool().await;
            let app = test_router(pool);

            let request = http::Request::builder()
                .method("GET")
                .uri("/app")
                .body(Body::empty())
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                response.headers().get("location").map(|v| v.to_str().unwrap()),
                Some("/login")
            );
        }

        #[tokio::test]
        async fn dashboard_renders_with_logout_form() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());
            let cookie = authenticated_cookie(&pool, &app, "dashboard@example.com", "Password123").await;

            let request = http::Request::builder()
                .method("GET")
                .uri("/app")
                .header("cookie", &cookie)
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

    mod password_reset {
        use crate::common::*;
        use axum::body::Body;
        use http::StatusCode;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        #[tokio::test]
        async fn forgot_password_unknown_email_returns_success() {
            let pool = test_pool().await;
            let app = test_router(pool);

            let body = forgot_password_form_body("unknown@example.com");
            let request = http::Request::builder()
                .method("POST")
                .uri("/forgot-password")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8_lossy(&body_bytes);
            assert!(
                body_str.contains("If an account exists for that email"),
                "Expected success message, got: {}",
                body_str
            );
        }

        #[tokio::test]
        async fn forgot_password_known_email_returns_success() {
            let pool = test_pool().await;
            let app = test_router(pool);

            // First create a user
            let signup_body = signup_form_body("reset-test@example.com", "Password123", "Password123");
            let signup_request = http::Request::builder()
                .method("POST")
                .uri("/signup")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(signup_body))
                .unwrap();
            let _ = app.clone().oneshot(signup_request).await.unwrap();

            // Now try forgot password
            let body = forgot_password_form_body("reset-test@example.com");
            let request = http::Request::builder()
                .method("POST")
                .uri("/forgot-password")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8_lossy(&body_bytes);
            assert!(
                body_str.contains("If an account exists for that email"),
                "Expected success message, got: {}",
                body_str
            );
        }

        #[tokio::test]
        async fn reset_password_invalid_token_redirects() {
            let pool = test_pool().await;
            let app = test_router(pool);

            let request = http::Request::builder()
                .method("GET")
                .uri("/reset-password?token=invalidtoken")
                .body(Body::empty())
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                response.headers().get("location").map(|v| v.to_str().unwrap()),
                Some("/forgot-password?error=invalid")
            );
        }

        #[tokio::test]
        async fn reset_password_missing_token_redirects() {
            let pool = test_pool().await;
            let app = test_router(pool);

            let request = http::Request::builder()
                .method("GET")
                .uri("/reset-password")
                .body(Body::empty())
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                response.headers().get("location").map(|v| v.to_str().unwrap()),
                Some("/forgot-password?error=invalid")
            );
        }
    }

    mod resend_verification {
        use crate::common::*;
        use axum::body::Body;
        use http::StatusCode;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        /// When validation fails (invalid email), the error page must preserve the `next`
        /// parameter in a hidden form field so the invite redirect flow is not broken.
        #[tokio::test]
        async fn invalid_email_preserves_next_in_error_response() {
            let pool = test_pool().await;
            let app = test_router(pool);

            let next = "/accept-invite/confirm?token=abc123";
            let body = resend_verification_form_body("not-an-email", Some(next));
            let request = http::Request::builder()
                .method("POST")
                .uri("/resend-verification")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8_lossy(&body_bytes);
            assert!(
                body_str.contains("Invalid email"),
                "Expected validation error message, got: {}",
                body_str
            );
            assert!(
                body_str.contains("name=\"next\"") && body_str.contains("value=\"/accept-invite/confirm?token=abc123\""),
                "Expected hidden next field to preserve invite redirect, got: {}",
                body_str
            );
        }

        /// GET with next param should render the form with hidden next field.
        #[tokio::test]
        async fn get_with_next_shows_hidden_next_in_form() {
            let pool = test_pool().await;
            let app = test_router(pool);

            let next = urlencoding::encode("/accept-invite/confirm?token=xyz");
            let request = http::Request::builder()
                .method("GET")
                .uri(&format!("/resend-verification?email=user@example.com&next={}", next))
                .body(Body::empty())
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8_lossy(&body_bytes);
            assert!(
                body_str.contains("name=\"next\"") && body_str.contains("/accept-invite/confirm"),
                "Expected hidden next field in form, got: {}",
                body_str
            );
        }

        /// Successful resend with next redirects to check-email with next preserved.
        #[tokio::test]
        async fn successful_resend_with_next_redirects_with_next_preserved() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());

            // Signup creates unverified user
            let signup_body = signup_form_body("resend-next@example.com", "Password123", "Password123");
            let signup_request = http::Request::builder()
                .method("POST")
                .uri("/signup")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(signup_body))
                .unwrap();
            let _ = app.clone().oneshot(signup_request).await.unwrap();

            let next = "/accept-invite/confirm?token=invite123";
            let body = resend_verification_form_body("resend-next@example.com", Some(next));
            let request = http::Request::builder()
                .method("POST")
                .uri("/resend-verification")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let location = response
                .headers()
                .get("location")
                .map(|v| v.to_str().unwrap())
                .unwrap();
            assert!(
                location.starts_with("/check-email"),
                "Expected redirect to check-email, got: {}",
                location
            );
            assert!(
                location.contains("next=") && location.contains("accept-invite"),
                "Expected next param preserved in redirect for invite flow, got: {}",
                location
            );
        }
    }

    mod account {
        use crate::common::*;
        use axum::body::Body;
        use http::StatusCode;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        #[tokio::test]
        async fn account_page_requires_authentication() {
            let pool = test_pool().await;
            let app = test_router(pool);

            let request = http::Request::builder()
                .method("GET")
                .uri("/app/account")
                .body(Body::empty())
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                response.headers().get("location").map(|v| v.to_str().unwrap()),
                Some("/login")
            );
        }

        #[tokio::test]
        async fn account_page_renders_with_email_and_form() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());
            let cookie =
                authenticated_cookie(&pool, &app, "account@example.com", "Password123").await;

            let request = http::Request::builder()
                .method("GET")
                .uri("/app/account")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8_lossy(&body_bytes);
            assert!(
                body_str.contains("account@example.com"),
                "Expected user email in account page, got: {}",
                body_str
            );
            assert!(
                body_str.contains("Change password"),
                "Expected change password form in account page, got: {}",
                body_str
            );
        }

        #[tokio::test]
        async fn change_password_requires_authentication() {
            let pool = test_pool().await;
            let app = test_router(pool);

            let body = change_password_form_body("Password123", "NewPassword123", "NewPassword123");
            let request = http::Request::builder()
                .method("POST")
                .uri("/app/account/change-password")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                response.headers().get("location").map(|v| v.to_str().unwrap()),
                Some("/login")
            );
        }

        #[tokio::test]
        async fn change_password_wrong_current_redirects_with_error() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());
            let cookie =
                authenticated_cookie(&pool, &app, "wrong-current@example.com", "Password123").await;

            let body = change_password_form_body("WrongPassword", "NewPassword123", "NewPassword123");
            let request = http::Request::builder()
                .method("POST")
                .uri("/app/account/change-password")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", &cookie)
                .body(Body::from(body))
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let location = response
                .headers()
                .get("location")
                .map(|v| v.to_str().unwrap())
                .unwrap();
            assert!(
                location.starts_with("/app/account?error="),
                "Expected redirect to account with error, got: {}",
                location
            );
        }

        #[tokio::test]
        async fn change_password_weak_new_password_redirects_with_error() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());
            let cookie =
                authenticated_cookie(&pool, &app, "weak-new@example.com", "Password123").await;

            let body = change_password_form_body("Password123", "weak", "weak");
            let request = http::Request::builder()
                .method("POST")
                .uri("/app/account/change-password")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", &cookie)
                .body(Body::from(body))
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let location = response
                .headers()
                .get("location")
                .map(|v| v.to_str().unwrap())
                .unwrap();
            assert!(
                location.starts_with("/app/account?error="),
                "Expected redirect with validation error, got: {}",
                location
            );
        }

        #[tokio::test]
        async fn change_password_mismatch_redirects_with_error() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());
            let cookie =
                authenticated_cookie(&pool, &app, "mismatch@example.com", "Password123").await;

            let body = change_password_form_body("Password123", "NewPassword123", "OtherPass456");
            let request = http::Request::builder()
                .method("POST")
                .uri("/app/account/change-password")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", &cookie)
                .body(Body::from(body))
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let location = response
                .headers()
                .get("location")
                .map(|v| v.to_str().unwrap())
                .unwrap();
            assert!(
                location.starts_with("/app/account?error="),
                "Expected redirect with mismatch error, got: {}",
                location
            );
        }

        #[tokio::test]
        async fn change_password_success_redirects_and_new_password_works() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());
            let cookie =
                authenticated_cookie(&pool, &app, "success@example.com", "Password123").await;

            let body = change_password_form_body("Password123", "NewPassword123", "NewPassword123");
            let request = http::Request::builder()
                .method("POST")
                .uri("/app/account/change-password")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", &cookie)
                .body(Body::from(body))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let location = response
                .headers()
                .get("location")
                .map(|v| v.to_str().unwrap())
                .unwrap();
            assert!(
                location.contains("success=password_changed"),
                "Expected redirect with success, got: {}",
                location
            );

            // Login with new password should succeed
            let login_body = login_form_body("success@example.com", "NewPassword123");
            let login_request = http::Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(login_body))
                .unwrap();
            let login_response = app.oneshot(login_request).await.unwrap();

            assert_eq!(login_response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                login_response
                    .headers()
                    .get("location")
                    .map(|v| v.to_str().unwrap()),
                Some("/app")
            );
        }

        #[tokio::test]
        async fn update_profile_image_requires_authentication() {
            let pool = test_pool().await;
            let app = test_router(pool);

            let body = update_profile_image_form_body("https://example.com/avatar.png");
            let request = http::Request::builder()
                .method("POST")
                .uri("/app/account/update-profile-image")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                response.headers().get("location").map(|v| v.to_str().unwrap()),
                Some("/login")
            );
        }

        #[tokio::test]
        async fn update_profile_image_https_required() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());
            let cookie =
                authenticated_cookie(&pool, &app, "http-url@example.com", "Password123").await;

            let body = update_profile_image_form_body("http://example.com/avatar.png");
            let request = http::Request::builder()
                .method("POST")
                .uri("/app/account/update-profile-image")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", &cookie)
                .body(Body::from(body))
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let location = response
                .headers()
                .get("location")
                .map(|v| v.to_str().unwrap())
                .unwrap();
            assert!(
                location.starts_with("/app/account?error="),
                "Expected redirect with error for non-HTTPS URL, got: {}",
                location
            );
        }

        #[tokio::test]
        async fn update_profile_image_requires_image_extension() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());
            let cookie =
                authenticated_cookie(&pool, &app, "no-ext@example.com", "Password123").await;

            let body = update_profile_image_form_body("https://example.com/document.pdf");
            let request = http::Request::builder()
                .method("POST")
                .uri("/app/account/update-profile-image")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", &cookie)
                .body(Body::from(body))
                .unwrap();
            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let location = response
                .headers()
                .get("location")
                .map(|v| v.to_str().unwrap())
                .unwrap();
            assert!(
                location.starts_with("/app/account?error="),
                "Expected redirect with error for non-image URL, got: {}",
                location
            );
        }

        #[tokio::test]
        async fn update_profile_image_success_and_displays_on_account_page() {
            let pool = test_pool().await;
            let app = test_router(pool.clone());
            let cookie =
                authenticated_cookie(&pool, &app, "avatar@example.com", "Password123").await;

            let url = "https://example.com/photo.jpg";
            let body = update_profile_image_form_body(url);
            let request = http::Request::builder()
                .method("POST")
                .uri("/app/account/update-profile-image")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", &cookie)
                .body(Body::from(body))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let location = response
                .headers()
                .get("location")
                .map(|v| v.to_str().unwrap())
                .unwrap();
            assert!(
                location.contains("success=profile_image_updated"),
                "Expected redirect with success, got: {}",
                location
            );

            let get_request = http::Request::builder()
                .method("GET")
                .uri("/app/account")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap();
            let get_response = app.oneshot(get_request).await.unwrap();
            assert_eq!(get_response.status(), StatusCode::OK);
            let body_bytes = get_response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8_lossy(&body_bytes);
            assert!(
                body_str.contains(url),
                "Expected profile image URL on account page, got: {}",
                body_str
            );
        }
    }
}
