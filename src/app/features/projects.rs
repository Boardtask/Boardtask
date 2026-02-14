use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use validator::Validate;

use crate::app::{db, AppState, APP_NAME};

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

/// Projects list template.
#[derive(Template)]
#[template(path = "projects_list.html")]
pub struct ProjectsListTemplate {
    pub app_name: &'static str,
    pub projects: Vec<db::projects::Project>,
}

/// Project detail template.
#[derive(Template)]
#[template(path = "projects_show.html")]
pub struct ProjectShowTemplate {
    pub app_name: &'static str,
    pub project: db::projects::Project,
}

/// Require valid session or redirect to login.
async fn require_session(
    jar: &CookieJar,
    pool: &sqlx::SqlitePool,
) -> Result<db::sessions::Session, Redirect> {
    let session_id = match jar.get("session_id") {
        Some(c) => c.value().to_string(),
        None => return Err(Redirect::to("/login")),
    };

    match db::sessions::find_valid(pool, &session_id).await {
        Ok(Some(s)) => Ok(s),
        Ok(None) | Err(_) => Err(Redirect::to("/login")),
    }
}

/// GET /app/projects/new — Show project creation form.
pub async fn show_form(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let _ = match require_session(&jar, &state.db).await {
        Ok(s) => s,
        Err(r) => return r.into_response(),
    };

    CreateProjectTemplate {
        app_name: APP_NAME,
        error: String::new(),
        title: String::new(),
    }
    .into_response()
}

/// POST /app/projects — Create project, redirect to list.
pub async fn create(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<CreateProjectForm>,
) -> Response {
    let session = match require_session(&jar, &state.db).await {
        Ok(s) => s,
        Err(r) => return r.into_response(),
    };

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

    let id = ulid::Ulid::new().to_string();
    let project = db::projects::NewProject {
        id: id.clone(),
        title: form.title.clone(),
        user_id: session.user_id.clone(),
    };

    if let Err(_) = db::projects::insert(&state.db, &project).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
            .into_response();
    }

    Redirect::to("/app/projects").into_response()
}

/// GET /app/projects — List user's projects.
pub async fn list(
    State(state): State<AppState>,
    jar: CookieJar,
) -> impl IntoResponse {
    let session = match require_session(&jar, &state.db).await {
        Ok(s) => s,
        Err(r) => return r.into_response(),
    };

    let projects = match db::projects::find_by_user_id(&state.db, &session.user_id).await {
        Ok(p) => p,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    ProjectsListTemplate {
        app_name: APP_NAME,
        projects,
    }
    .into_response()
}

/// GET /app/projects/:id — Show project detail.
pub async fn show(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let session = match require_session(&jar, &state.db).await {
        Ok(s) => s,
        Err(r) => return r.into_response(),
    };

    let project = match db::projects::find_by_id(&state.db, &id).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "Project not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    // Ensure user owns the project
    if project.user_id != session.user_id {
        return (StatusCode::NOT_FOUND, "Project not found").into_response();
    }

    ProjectShowTemplate {
        app_name: APP_NAME,
        project,
    }
    .into_response()
}

/// Projects routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/app/projects/new", get(show_form))
        .route("/app/projects", get(list).post(create))
        .route("/app/projects/:id", get(show))
}
