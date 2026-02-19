use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use validator::Validate;

use crate::app::{
    db,
    session::AuthenticatedSession,
    tenant,
    AppState, APP_NAME,
};

/// Create project form data.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProjectForm {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
}

/// Project creation form template.
#[derive(Template)]
#[template(path = "projects_create.html")]
pub struct CreateProjectTemplate {
    pub app_name: &'static str,
    pub error: String,
    pub title: String,
}

/// GET /app/projects/new — Show project creation form.
pub async fn show_form(
    AuthenticatedSession(_): AuthenticatedSession,
    State(_state): State<AppState>,
) -> Response {
    CreateProjectTemplate {
        app_name: APP_NAME,
        error: String::new(),
        title: String::new(),
    }
    .into_response()
}

/// POST /app/projects — Create project, redirect to list.
pub async fn create(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Form(form): Form<CreateProjectForm>,
) -> Response {
    if form.validate().is_err() {
        let template = CreateProjectTemplate {
            app_name: APP_NAME,
            error: "Title must be 1–255 characters".to_string(),
            title: form.title.clone(),
        };
        return Html(
            template
                .render()
                .unwrap_or_else(|_| "Template error".to_string()),
        )
        .into_response();
    }

    // Validate org membership on every write - never trust session
    if tenant::require_org_member(&state.db, &session.user_id, &session.organization_id)
        .await
        .is_err()
    {
        return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response();
    }

    let id = ulid::Ulid::new().to_string();
    let organization_id = session.organization_id.clone();

    let project = db::projects::NewProject {
        id: id.clone(),
        title: form.title.clone(),
        user_id: session.user_id.clone(),
        organization_id,
    };

    if db::projects::insert(&state.db, &project).await.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
            .into_response();
    }

    Redirect::to("/app/projects").into_response()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/app/projects/new", get(show_form))
        .route("/app/projects", post(create))
}
