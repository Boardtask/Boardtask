use sqlx::FromRow;
use time::OffsetDateTime;

/// Database row for projects table.
#[derive(Debug, FromRow)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub user_id: String,
    pub created_at: i64,
}

/// Data structure for inserting a new project.
pub struct NewProject {
    pub id: String,
    pub title: String,
    pub user_id: String,
}

/// Insert a new project into the database.
pub async fn insert<'e, E>(
    executor: E,
    project: &NewProject,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "INSERT INTO projects (id, title, user_id, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&project.id)
    .bind(&project.title)
    .bind(&project.user_id)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(())
}

/// Find all projects for a user.
pub async fn find_by_user_id(
    pool: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, title, user_id, created_at FROM projects WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Find a project by ID.
pub async fn find_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, title, user_id, created_at FROM projects WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
