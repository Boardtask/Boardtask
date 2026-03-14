use sqlx::{FromRow, SqliteExecutor};
use time::OffsetDateTime;

use crate::app::domain::UserId;

/// Database row for team_members table.
#[derive(Debug, FromRow)]
pub struct TeamMember {
    pub team_id: String,
    pub user_id: String,
    pub created_at: i64,
}

/// Data structure for inserting a new team member.
pub struct NewTeamMember {
    pub team_id: String,
    pub user_id: String,
}

/// Insert a new team member. Use add_member for idempotent "ensure in team".
pub async fn insert<'e, E>(executor: E, member: &NewTeamMember) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query(
        "INSERT INTO team_members (team_id, user_id, created_at) VALUES (?, ?, ?)",
    )
    .bind(&member.team_id)
    .bind(&member.user_id)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

/// Add a user to a team. Idempotent: no-op if already a member (INSERT OR IGNORE).
pub async fn add_member<'e, E>(
    executor: E,
    team_id: &str,
    user_id: &UserId,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query(
        "INSERT OR IGNORE INTO team_members (team_id, user_id, created_at) VALUES (?, ?, ?)",
    )
    .bind(team_id)
    .bind(user_id.as_str())
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

/// Check if a user is a member of a team.
pub async fn is_member<'e, E>(
    executor: E,
    team_id: &str,
    user_id: &UserId,
) -> Result<bool, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM team_members WHERE team_id = ? AND user_id = ?",
    )
    .bind(team_id)
    .bind(user_id.as_str())
    .fetch_one(executor)
    .await?;
    Ok(count > 0)
}

/// Profile URL for a team member (for avatar display).
#[derive(Debug, FromRow)]
pub struct TeamMemberAvatar {
    pub profile_image_url: Option<String>,
}

/// List profile image URLs for team members (for project list Team column).
/// Returns up to 7 URLs; empty string for members without an image (placeholder).
pub async fn list_avatar_urls_for_team<'e, E>(
    executor: E,
    team_id: &str,
) -> Result<Vec<String>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let rows = sqlx::query_as::<_, TeamMemberAvatar>(
        "SELECT u.profile_image_url FROM team_members tm JOIN users u ON u.id = tm.user_id WHERE tm.team_id = ? ORDER BY tm.created_at LIMIT 7",
    )
    .bind(team_id)
    .fetch_all(executor)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| r.profile_image_url.unwrap_or_default())
        .collect())
}

/// Count of team members (for "+N" overflow display).
pub async fn count_by_team<'e, E>(executor: E, team_id: &str) -> Result<i64, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_scalar("SELECT COUNT(*) FROM team_members WHERE team_id = ?")
        .bind(team_id)
        .fetch_one(executor)
        .await
}
