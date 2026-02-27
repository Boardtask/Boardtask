use sqlx::{FromRow, SqliteExecutor};
use time::OffsetDateTime;

use crate::app::domain::OrganizationId;

/// Database row for teams table.
#[derive(Debug, Clone, FromRow)]
pub struct Team {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub created_at: i64,
}

/// Data structure for inserting a new team.
pub struct NewTeam {
    pub id: String,
    pub organization_id: String,
    pub name: String,
}

/// Insert a new team into the database.
pub async fn insert<'e, E>(executor: E, team: &NewTeam) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query(
        "INSERT INTO teams (id, organization_id, name, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&team.id)
    .bind(&team.organization_id)
    .bind(&team.name)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

/// Find a team by ID.
pub async fn find_by_id<'e, E>(
    executor: E,
    id: &str,
) -> Result<Option<Team>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, Team>(
        "SELECT id, organization_id, name, created_at FROM teams WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(executor)
    .await
}

/// Find all teams for an organization, ordered by created_at.
pub async fn find_by_organization<'e, E>(
    executor: E,
    organization_id: &str,
) -> Result<Vec<Team>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, Team>(
        "SELECT id, organization_id, name, created_at FROM teams WHERE organization_id = ? ORDER BY created_at",
    )
    .bind(organization_id)
    .fetch_all(executor)
    .await
}

/// Return the default team for an organization (first by created_at). Used when adding users to default team and for project creation.
pub async fn find_default_for_org<'e, E>(
    executor: E,
    organization_id: &OrganizationId,
) -> Result<Option<Team>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, Team>(
        "SELECT id, organization_id, name, created_at FROM teams WHERE organization_id = ? ORDER BY created_at LIMIT 1",
    )
    .bind(organization_id.as_str())
    .fetch_optional(executor)
    .await
}
