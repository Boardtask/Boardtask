use sqlx::{FromRow, SqliteExecutor};
use time::OffsetDateTime;

use crate::app::domain::{OrganizationId, OrganizationRole, UserId};

/// Database row for organizations table.
#[derive(Debug, FromRow)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

/// Data structure for inserting a new organization.
pub struct NewOrganization {
    pub id: OrganizationId,
    pub name: String,
}

/// Find an organization by ID.
pub async fn find_by_id<'e, E>(
    executor: E,
    organization_id: &OrganizationId,
) -> Result<Option<Organization>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, Organization>(
        "SELECT id, name, created_at FROM organizations WHERE id = ?",
    )
    .bind(organization_id.as_str())
    .fetch_optional(executor)
    .await
}

/// Insert a new organization.
pub async fn insert<'e, E>(
    executor: E,
    organization: &NewOrganization,
) -> Result<(), sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("INSERT INTO organizations (id, name, created_at) VALUES (?, ?, ?)")
        .bind(organization.id.as_str())
        .bind(&organization.name)
        .bind(now)
        .execute(executor)
        .await?;
    Ok(())
}

/// Add a user to an organization with a specific role.
pub async fn add_member<'e, E>(
    executor: E,
    organization_id: &OrganizationId,
    user_id: &UserId,
    role: OrganizationRole,
) -> Result<(), sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query(
        "INSERT INTO organization_members (organization_id, user_id, role, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(organization_id.as_str())
    .bind(user_id.as_str())
    .bind(role.to_string())
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

/// Check if a user is a member of an organization.
pub async fn is_member<'e, E>(
    executor: E,
    organization_id: &OrganizationId,
    user_id: &UserId,
) -> Result<bool, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM organization_members WHERE organization_id = ? AND user_id = ?",
    )
    .bind(organization_id.as_str())
    .bind(user_id.as_str())
    .fetch_one(executor)
    .await?;

    Ok(count > 0)
}

/// Find a member's role in an organization. Returns None if not a member.
pub async fn find_member_role<'e, E>(
    executor: E,
    organization_id: &OrganizationId,
    user_id: &UserId,
) -> Result<Option<OrganizationRole>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let row: Option<String> = sqlx::query_scalar(
        "SELECT role FROM organization_members WHERE organization_id = ? AND user_id = ?",
    )
    .bind(organization_id.as_str())
    .bind(user_id.as_str())
    .fetch_optional(executor)
    .await?;

    Ok(row.and_then(|r| r.parse::<OrganizationRole>().ok()))
}
