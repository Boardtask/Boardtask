use crate::app::{db, error::AppError};

/// Ensure a user owns a project. Returns NotFound if missing or not owned.
pub async fn ensure_project_owned(
    pool: &sqlx::SqlitePool,
    project_id: &str,
    user_id: &str,
    organization_id: &str,
) -> Result<db::projects::Project, AppError> {
    let project = db::projects::find_by_id(pool, project_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

    if project.user_id != user_id || project.organization_id != organization_id {
        return Err(AppError::NotFound("Project not found".to_string()));
    }

    Ok(project)
}