//! GET /api/projects/:project_id/export — Download project as JSON.

use axum::{
    extract::{Path, State},
    http::{
        header::{HeaderValue, CONTENT_DISPOSITION, CONTENT_TYPE},
        StatusCode,
    },
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use crate::app::{
    db,
    error::AppError,
    features::graph,
    features::projects::import_export::{
        ProjectExport, ProjectExportEdge, ProjectExportNode, ProjectExportProject,
        ProjectExportSlot, EXPORT_VERSION,
    },
    session::ApiAuthenticatedSession,
    AppState,
};
use time::OffsetDateTime;

/// Sanitize project title for use in filename: keep alphanumeric and spaces, replace rest with underscore.
fn sanitize_filename_title(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    s.trim().chars().take(80).collect::<String>().trim().to_string()
}

/// GET /api/projects/:project_id/export — Export project as JSON attachment.
pub async fn export_project(
    ApiAuthenticatedSession(session): ApiAuthenticatedSession,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Response, AppError> {
    let project = graph::helpers::ensure_project_accessible(&state.db, &project_id, &session.user_id).await?;

    let (slots, nodes, edges) = tokio::try_join!(
        db::project_slots::find_by_project(&state.db, &project_id),
        db::nodes::find_by_project(&state.db, &project_id),
        db::node_edges::find_by_project(&state.db, &project_id),
    )?;

    let exported_at = OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339).ok().map(String::from);

    let payload = ProjectExport {
        version: EXPORT_VERSION,
        exported_at,
        project: ProjectExportProject {
            title: project.title.clone(),
        },
        slots: slots
            .into_iter()
            .map(|s| ProjectExportSlot {
                id: s.id,
                name: s.name,
                sort_order: s.sort_order,
            })
            .collect(),
        nodes: nodes
            .into_iter()
            .map(|n| ProjectExportNode {
                id: n.id,
                node_type_id: n.node_type_id,
                status_id: n.status_id,
                title: n.title,
                description: n.description,
                estimated_minutes: n.estimated_minutes,
                slot_id: n.slot_id,
                parent_id: n.parent_id,
                assigned_user_id: n.assigned_user_id,
            })
            .collect(),
        edges: edges
            .into_iter()
            .map(|e| ProjectExportEdge {
                parent_id: e.parent_id,
                child_id: e.child_id,
            })
            .collect(),
    };

    let body = serde_json::to_vec(&payload).map_err(|_| AppError::Internal)?;

    let filename = sanitize_filename_title(&project.title);
    let safe_name = if filename.is_empty() {
        "project".to_string()
    } else {
        format!("project-{}", filename.replace(' ', "-"))
    };
    let disposition = format!(r#"attachment; filename="{}.json""#, safe_name);
    let disposition_value = HeaderValue::try_from(disposition)
        .unwrap_or_else(|_| HeaderValue::from_static("attachment; filename=project.json"));

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(CONTENT_DISPOSITION, disposition_value);

    Ok((StatusCode::OK, headers, body).into_response())
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/projects/:project_id/export", get(export_project))
}
