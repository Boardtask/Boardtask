use sqlx::FromRow;
use time::OffsetDateTime;

use crate::app::domain::{Email, HashedPassword, OrganizationId, ProfileImageUrl, UserId};

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
    pub first_name: String,
    pub last_name: String,
    pub profile_image_url: Option<String>,
}

/// Data structure for inserting a new user.
pub struct NewUser {
    pub id: UserId,
    pub email: Email,
    pub password_hash: HashedPassword,
    pub organization_id: OrganizationId,
    pub first_name: String,
    pub last_name: String,
}

/// Find a user by email address.
pub async fn find_by_email(
    pool: &sqlx::SqlitePool,
    email: &Email,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at, updated_at, email_verified_at, organization_id, COALESCE(first_name, '') AS first_name, COALESCE(last_name, '') AS last_name, profile_image_url FROM users WHERE email = ?",
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
    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at, updated_at, email_verified_at, organization_id, COALESCE(first_name, '') AS first_name, COALESCE(last_name, '') AS last_name, profile_image_url FROM users WHERE id = ?",
    )
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

/// Update a user's first and last name.
pub async fn update_name<'e, E>(
    executor: E,
    user_id: &UserId,
    first_name: &str,
    last_name: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("UPDATE users SET first_name = ?, last_name = ?, updated_at = ? WHERE id = ?")
        .bind(first_name)
        .bind(last_name)
        .bind(now)
        .bind(user_id.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

/// Update a user's profile image URL. Pass None to clear.
pub async fn update_profile_image_url<'e, E>(
    executor: E,
    user_id: &UserId,
    url: Option<&ProfileImageUrl>,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let value = url.map(ProfileImageUrl::as_str);
    sqlx::query("UPDATE users SET profile_image_url = ?, updated_at = ? WHERE id = ?")
        .bind(value)
        .bind(now)
        .bind(user_id.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

/// Return display name: "First Last" if either name is set, otherwise email.
pub fn display_name(user: &User) -> String {
    display_name_from_parts(&user.first_name, &user.last_name, &user.email)
}

/// Display name from first/last and email fallback.
pub fn display_name_from_parts(first_name: &str, last_name: &str, email: &str) -> String {
    let first = first_name.trim();
    let last = last_name.trim();
    let full = format!("{} {}", first, last).trim().to_string();
    if full.is_empty() {
        email.to_string()
    } else {
        full
    }
}

/// Update a user's organization (e.g. when accepting an invite to a different org).
pub async fn update_organization_id<'e, E>(
    executor: E,
    user_id: &UserId,
    organization_id: &OrganizationId,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("UPDATE users SET organization_id = ?, updated_at = ? WHERE id = ?")
        .bind(organization_id.as_str())
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
        "INSERT INTO users (id, email, password_hash, organization_id, created_at, updated_at, email_verified_at, first_name, last_name) VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)",
    )
    .bind(user.id.as_str())
    .bind(user.email.as_str())
    .bind(user.password_hash.as_str())
    .bind(user.organization_id.as_str())
    .bind(now)
    .bind(now)
    .bind(&user.first_name)
    .bind(&user.last_name)
    .execute(executor)
    .await?;

    Ok(())
}