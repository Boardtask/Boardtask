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
    pub bio: Option<String>,
    pub email_notifications: i32,
    pub theme_mode: String,
    pub language: String,
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
        "SELECT id, email, password_hash, created_at, updated_at, email_verified_at, organization_id, COALESCE(first_name, '') AS first_name, COALESCE(last_name, '') AS last_name, profile_image_url, bio, COALESCE(email_notifications, 1) AS email_notifications, COALESCE(theme_mode, 'light') AS theme_mode, COALESCE(language, 'en-US') AS language FROM users WHERE email = ?",
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
        "SELECT id, email, password_hash, created_at, updated_at, email_verified_at, organization_id, COALESCE(first_name, '') AS first_name, COALESCE(last_name, '') AS last_name, profile_image_url, bio, COALESCE(email_notifications, 1) AS email_notifications, COALESCE(theme_mode, 'light') AS theme_mode, COALESCE(language, 'en-US') AS language FROM users WHERE id = ?",
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

/// Update a user's bio.
pub async fn update_bio<'e, E>(
    executor: E,
    user_id: &UserId,
    bio: Option<&str>,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let value = bio.and_then(|s| if s.trim().is_empty() { None } else { Some(s.trim()) });
    sqlx::query("UPDATE users SET bio = ?, updated_at = ? WHERE id = ?")
        .bind(value)
        .bind(now)
        .bind(user_id.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

/// Update a user's preferences.
pub async fn update_preferences<'e, E>(
    executor: E,
    user_id: &UserId,
    email_notifications: bool,
    theme_mode: &str,
    language: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("UPDATE users SET email_notifications = ?, theme_mode = ?, language = ?, updated_at = ? WHERE id = ?")
        .bind(if email_notifications { 1 } else { 0 })
        .bind(theme_mode)
        .bind(language)
        .bind(now)
        .bind(user_id.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

/// Delete a user by ID. Cascades to sessions, organization_members, etc.
pub async fn delete<'e, E>(executor: E, user_id: &UserId) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query("DELETE FROM users WHERE id = ?")
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

/// Return profile image URL for a user, or empty string if none.
pub async fn profile_image_url_for<'e, E>(
    executor: E,
    user_id: &UserId,
) -> String
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    find_by_id(executor, user_id)
        .await
        .ok()
        .flatten()
        .and_then(|u| u.profile_image_url)
        .unwrap_or_default()
}

/// Return display name: "First Last" (first_name and last_name are always non-empty).
pub fn display_name(user: &User) -> String {
    display_name_from_parts(&user.first_name, &user.last_name)
}

/// Display name from first and last name (both are required and non-empty).
pub fn display_name_from_parts(first_name: &str, last_name: &str) -> String {
    format!("{} {}", first_name.trim(), last_name.trim()).trim().to_string()
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