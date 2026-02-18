use sqlx::FromRow;
use time::OffsetDateTime;

use crate::app::domain::{Email, HashedPassword, OrganizationId, UserId};

/// Database row for users table.
#[derive(Debug, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub email_verified_at: Option<i64>,
    pub organization_id: String,
}

/// Data structure for inserting a new user.
pub struct NewUser {
    pub id: UserId,
    pub email: Email,
    pub password_hash: HashedPassword,
    pub organization_id: OrganizationId,
}

/// Find a user by email address.
pub async fn find_by_email(
    pool: &sqlx::SqlitePool,
    email: &Email,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at, updated_at, email_verified_at, organization_id FROM users WHERE email = ?",
    )
    .bind(email.as_str())
    .fetch_optional(pool)
    .await
}

/// Find a user by ID.
pub async fn find_by_id<'e, E>(
    executor: E,
    user_id: &UserId,
) -> Result<Option<User>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(user_id.as_str())
        .fetch_optional(executor)
        .await
}

/// Mark a user's email as verified.
pub async fn mark_verified<'e, E>(
    executor: E,
    user_id: &UserId,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("UPDATE users SET email_verified_at = ?, updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(now)
        .bind(user_id.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

/// Update a user's password hash.
pub async fn update_password<'e, E>(
    executor: E,
    user_id: &UserId,
    password_hash: &HashedPassword,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
        .bind(password_hash.as_str())
        .bind(now)
        .bind(user_id.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

/// Insert a new user into the database.
pub async fn insert<'e, E>(
    executor: E,
    user: &NewUser,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, organization_id, created_at, updated_at, email_verified_at) VALUES (?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(user.id.as_str())
    .bind(user.email.as_str())
    .bind(user.password_hash.as_str())
    .bind(user.organization_id.as_str())
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(())
}