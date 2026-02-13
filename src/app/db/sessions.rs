use sqlx::FromRow;
use time::OffsetDateTime;

use crate::app::domain::UserId;

/// Database row for sessions table.
#[derive(Debug, FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub expires_at: i64,
    pub created_at: i64,
}

/// Create a new session for a user. Returns the session ID.
pub async fn create<'e, E>(
    executor: E,
    user_id: &UserId,
    expires_at: OffsetDateTime,
) -> Result<String, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    use crate::app::domain::UserId;

    let session_id = UserId::new().as_str();
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "INSERT INTO sessions (id, user_id, expires_at, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&session_id)
    .bind(user_id.as_str())
    .bind(expires_at.unix_timestamp())
    .bind(now)
    .execute(executor)
    .await?;

    Ok(session_id)
}

/// Find a valid (non-expired) session by ID.
pub async fn find_valid(
    pool: &sqlx::SqlitePool,
    session_id: &str,
) -> Result<Option<Session>, sqlx::Error> {
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query_as::<_, Session>(
        "SELECT id, user_id, expires_at, created_at FROM sessions WHERE id = ? AND expires_at > ?",
    )
    .bind(session_id)
    .bind(now)
    .fetch_optional(pool)
    .await
}