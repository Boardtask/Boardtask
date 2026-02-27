use sqlx::{FromRow, SqliteExecutor};
use time::OffsetDateTime;

use crate::app::domain::{OrganizationId, OrganizationRole, UserId};

/// Database row for organization_invites table.
#[derive(Debug, FromRow)]
pub struct OrganizationInvite {
    pub id: String,
    pub organization_id: String,
    pub email: String,
    pub role: String,
    pub invited_by_user_id: String,
    pub token: String,
    pub expires_at: i64,
    pub created_at: i64,
}

/// Data structure for inserting a new organization invite.
pub struct NewOrganizationInvite {
    pub id: String,
    pub organization_id: OrganizationId,
    pub email: String,
    pub role: OrganizationRole,
    pub invited_by_user_id: UserId,
    pub token: String,
    pub expires_at: i64,
    pub created_at: i64,
}

/// Insert a new organization invite.
pub async fn insert<'e, E>(
    executor: E,
    invite: &NewOrganizationInvite,
) -> Result<(), sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query(
        "INSERT INTO organization_invites (id, organization_id, email, role, invited_by_user_id, token, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&invite.id)
    .bind(invite.organization_id.as_str())
    .bind(&invite.email)
    .bind(invite.role.to_string())
    .bind(invite.invited_by_user_id.as_str())
    .bind(&invite.token)
    .bind(invite.expires_at)
    .bind(invite.created_at)
    .execute(executor)
    .await?;
    Ok(())
}

/// Find an invite by token. Returns the invite if it exists and has not expired.
/// Does not return invites that have been deleted (after accept).
pub async fn find_by_token<'e, E>(
    executor: E,
    token: &str,
) -> Result<Option<OrganizationInvite>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query_as::<_, OrganizationInvite>(
        "SELECT id, organization_id, email, role, invited_by_user_id, token, expires_at, created_at FROM organization_invites WHERE token = ? AND expires_at > ?",
    )
    .bind(token)
    .bind(now)
    .fetch_optional(executor)
    .await
}

/// Delete an invite (e.g. after it has been accepted). Use this to "mark accepted".
pub async fn delete_by_id<'e, E>(executor: E, id: &str) -> Result<(), sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query("DELETE FROM organization_invites WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}

/// Find a pending invite for the same org and email (for duplicate check).
pub async fn find_pending_by_org_and_email<'e, E>(
    executor: E,
    organization_id: &OrganizationId,
    email: &str,
) -> Result<Option<OrganizationInvite>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query_as::<_, OrganizationInvite>(
        "SELECT id, organization_id, email, role, invited_by_user_id, token, expires_at, created_at FROM organization_invites WHERE organization_id = ? AND email = ? AND expires_at > ?",
    )
    .bind(organization_id.as_str())
    .bind(email)
    .bind(now)
    .fetch_optional(executor)
    .await
}

/// List pending invites for an organization (for settings page).
pub async fn list_pending_for_org<'e, E>(
    executor: E,
    organization_id: &OrganizationId,
) -> Result<Vec<OrganizationInvite>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query_as::<_, OrganizationInvite>(
        "SELECT id, organization_id, email, role, invited_by_user_id, token, expires_at, created_at FROM organization_invites WHERE organization_id = ? AND expires_at > ? ORDER BY created_at DESC",
    )
    .bind(organization_id.as_str())
    .bind(now)
    .fetch_all(executor)
    .await
}
