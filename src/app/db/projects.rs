use std::str::FromStr;

use sqlx::FromRow;
use time::OffsetDateTime;

use crate::app::domain::ProjectViewMode;

/// Database row for projects table.
#[derive(Debug, FromRow)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub user_id: String,
    pub created_at: i64,
    pub organization_id: String,
    pub team_id: Option<String>,
    pub default_view_mode: String,
}

impl Project {
    /// Returns the default view mode as a domain type. Falls back to Graph for invalid values.
    pub fn default_view_mode(&self) -> ProjectViewMode {
        ProjectViewMode::from_str(&self.default_view_mode).unwrap_or(ProjectViewMode::Graph)
    }
}

/// Data structure for inserting a new project.
pub struct NewProject {
    pub id: String,
    pub title: String,
    pub user_id: String,
    pub organization_id: String,
    pub team_id: String,
}

/// Insert a new project into the database.
pub async fn insert<'e, E>(
    executor: E,
    project: &NewProject,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "INSERT INTO projects (id, title, user_id, created_at, organization_id, team_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&project.id)
    .bind(&project.title)
    .bind(&project.user_id)
    .bind(now)
    .bind(&project.organization_id)
    .bind(&project.team_id)
    .execute(executor)
    .await?;

    Ok(())
}

/// Delete a project by ID and organization ID (tenant-scoped).
pub async fn delete_by_id_and_org<'e, E>(
    executor: E,
    project_id: &str,
    organization_id: &str,
) -> Result<bool, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let result = sqlx::query("DELETE FROM projects WHERE id = ? AND organization_id = ?")
        .bind(project_id)
        .bind(organization_id)
        .execute(executor)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Find all projects for a user scoped by organisation.
pub async fn find_by_user_and_org(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    organization_id: &str,
) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, title, user_id, created_at, organization_id, team_id, default_view_mode FROM projects WHERE user_id = ? AND organization_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .bind(organization_id)
    .fetch_all(pool)
    .await
}

/// List all projects for an organisation. Caller must have verified org membership.
pub async fn list_for_org(
    pool: &sqlx::SqlitePool,
    organization_id: &str,
) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, title, user_id, created_at, organization_id, team_id, default_view_mode FROM projects WHERE organization_id = ? ORDER BY created_at DESC",
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
}

/// Find a project by ID and organisation. Returns None if project doesn't exist or belongs to another org.
pub async fn find_by_id_and_org(
    pool: &sqlx::SqlitePool,
    id: &str,
    organization_id: &str,
) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, title, user_id, created_at, organization_id, team_id, default_view_mode FROM projects WHERE id = ? AND organization_id = ?",
    )
    .bind(id)
    .bind(organization_id)
    .fetch_optional(pool)
    .await
}

/// Find a project by ID.
pub async fn find_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, title, user_id, created_at, organization_id, team_id, default_view_mode FROM projects WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Update default_view_mode for a project. Validates org ownership. Returns the updated project.
pub async fn update_default_view_mode(
    pool: &sqlx::SqlitePool,
    project_id: &str,
    organization_id: &str,
    value: ProjectViewMode,
) -> Result<Option<Project>, sqlx::Error> {
    let value_str = value.to_string();
    let result = sqlx::query(
        "UPDATE projects SET default_view_mode = ? WHERE id = ? AND organization_id = ?",
    )
    .bind(&value_str)
    .bind(project_id)
    .bind(organization_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }

    find_by_id_and_org(pool, project_id, organization_id).await
}

