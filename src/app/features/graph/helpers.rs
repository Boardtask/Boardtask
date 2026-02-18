use std::borrow::Cow;

use validator::ValidationError;

use crate::app::{db, error::AppError};

/// Default task status ID (system "To do"). Must match migration INSERT.
pub const DEFAULT_STATUS_ID: &str = "01JSTATUS00000000TODO0000";

/// Max estimated minutes (avoids 10^18, malicious input, keeps aggregation safe).
pub const MAX_ESTIMATED_MINUTES: i64 = 1_000_000_000;

/// Validates estimated_minutes for use with the validator crate (create/update node requests).
pub fn validate_estimated_minutes(value: i64) -> Result<(), ValidationError> {
    if value < 0 || value > MAX_ESTIMATED_MINUTES {
        return Err(ValidationError::new("estimated_minutes")
            .with_message(Cow::Borrowed("must be between 0 and 1_000_000_000")));
    }
    Ok(())
}

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