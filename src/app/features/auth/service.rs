use time::Duration;
use sqlx::SqlitePool;

use crate::app::{
    domain::{Email, Password, HashedPassword, UserId},
    db,
    error::AppError,
};

/// Sign up a new user. Returns the session ID on success.
pub async fn signup(
    pool: &SqlitePool,
    email: &Email,
    password: &Password,
) -> Result<String, AppError> {
    // Check if email already exists
    if let Some(_) = db::find_by_email(pool, email).await.map_err(AppError::Database)? {
        return Err(AppError::Auth("Unable to create account. If you already have an account, please log in.".to_string()));
    }

    // Hash the password
    let password_hash = HashedPassword::from_password(password)
        .map_err(|_| AppError::Internal)?;

    // Generate user ID
    let user_id = UserId::new();

    // Create new user
    let new_user = db::NewUser {
        id: user_id,
        email: email.clone(),
        password_hash,
    };

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    db::insert(&mut *tx, &new_user).await.map_err(AppError::Database)?;

    // Create session (30 days)
    let expires_at = time::OffsetDateTime::now_utc() + Duration::days(30);
    let session_id = db::create(&mut *tx, &new_user.id, expires_at)
        .await
        .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    Ok(session_id)
}

/// Log in a user. Returns the session ID on success.
pub async fn login(
    pool: &SqlitePool,
    email: &Email,
    password: &Password,
) -> Result<String, AppError> {
    // Find user by email
    let user = db::find_by_email(pool, email)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::Auth("Invalid email or password".to_string()))?;

    // Verify password
    let stored_hash = HashedPassword::from_string(user.password_hash);
    stored_hash.verify(password)
        .map_err(|_| AppError::Auth("Invalid email or password".to_string()))?;

    // Parse user ID
    let user_id = UserId::from_string(&user.id)
        .map_err(|_| AppError::Internal)?;

    // Create session (30 days)
    let expires_at = time::OffsetDateTime::now_utc() + Duration::days(30);
    let session_id = db::create(pool, &user_id, expires_at)
        .await
        .map_err(AppError::Database)?;

    Ok(session_id)
}