use time::OffsetDateTime;

use crate::app::domain::UserId;

/// Insert a verification token for a user.
pub async fn insert_token<'e, E>(
    executor: E,
    user_id: &UserId,
    token: &str,
    expires_at: OffsetDateTime,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let id = UserId::new().as_str();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query(
        "INSERT INTO email_verification_tokens (id, user_id, token, expires_at, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(user_id.as_str())
    .bind(token)
    .bind(expires_at.unix_timestamp())
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

/// Find a valid (non-expired) token. Returns user_id if found.
pub async fn find_valid_token(
    pool: &sqlx::SqlitePool,
    token: &str,
) -> Result<Option<UserId>, sqlx::Error> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let row = sqlx::query_scalar::<_, String>(
        "SELECT user_id FROM email_verification_tokens WHERE token = ? AND expires_at > ?",
    )
    .bind(token)
    .bind(now)
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(|s| UserId::from_string(&s).ok()))
}

/// Find a valid verification token for a user (e.g. for tests). Returns the token string if found.
pub async fn find_token_for_user(
    pool: &sqlx::SqlitePool,
    user_id: &UserId,
) -> Result<Option<String>, sqlx::Error> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let row = sqlx::query_scalar::<_, String>(
        "SELECT token FROM email_verification_tokens WHERE user_id = ? AND expires_at > ?",
    )
    .bind(user_id.as_str())
    .bind(now)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Delete all verification tokens for a user (used before resending a new link).
pub async fn delete_tokens_for_user<'e, E>(executor: E, user_id: &UserId) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query("DELETE FROM email_verification_tokens WHERE user_id = ?")
        .bind(user_id.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

/// Delete a token after successful verification.
pub async fn delete_token<'e, E>(executor: E, token: &str) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query("DELETE FROM email_verification_tokens WHERE token = ?")
        .bind(token)
        .execute(executor)
        .await?;
    Ok(())
}