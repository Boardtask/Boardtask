use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::post,
    Router,
};

use crate::app::{
    db,
    session::AuthenticatedSession,
    tenant,
    AppState,
};

/// DELETE /api/projects/:id — Delete a project (tenant-scoped).
pub async fn delete(
    AuthenticatedSession(session): AuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    // Validate org membership - scope every write
    if tenant::require_org_member(&state.db, &session.user_id, &session.organization_id)
        .await
        .is_err()
    {
        return (StatusCode::NOT_FOUND, "Not found".to_string()).into_response();
    }

    // Check if project exists and belongs to user's org
    let project_exists = match db::projects::find_by_id_and_org(&state.db, &project_id, &session.organization_id).await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    if !project_exists {
        return (StatusCode::NOT_FOUND, "Project not found".to_string()).into_response();
    }

    // Delete the project
    match db::projects::delete_by_id_and_org(&state.db, &project_id, &session.organization_id).await {
        Ok(true) => {
            // Success - redirect back to projects list
            Redirect::to("/app/projects").into_response()
        }
        Ok(false) => {
            // Project was not found or didn't belong to org (shouldn't happen due to check above)
            (StatusCode::NOT_FOUND, "Project not found".to_string()).into_response()
        }
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/projects/:id/delete", post(delete))
}