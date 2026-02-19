use sqlx::FromRow;
use time::OffsetDateTime;

/// Database row for projects table.
#[derive(Debug, FromRow)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub user_id: String,
    pub created_at: i64,
    pub organization_id: String,
}

/// Data structure for inserting a new project.
pub struct NewProject {
    pub id: String,
    pub title: String,
    pub user_id: String,
    pub organization_id: String,
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
        "INSERT INTO projects (id, title, user_id, created_at, organization_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&project.id)
    .bind(&project.title)
    .bind(&project.user_id)
    .bind(now)
    .bind(&project.organization_id)
    .execute(executor)
    .await?;

    Ok(())
}

/// Find all projects for a user scoped by organisation.
pub async fn find_by_user_and_org(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    organization_id: &str,
) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, title, user_id, created_at, organization_id FROM projects WHERE user_id = ? AND organization_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .bind(organization_id)
    .fetch_all(pool)
    .await
}

/// Find a project by ID and organisation. Returns None if project doesn't exist or belongs to another org.
pub async fn find_by_id_and_org(
    pool: &sqlx::SqlitePool,
    id: &str,
    organization_id: &str,
) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, title, user_id, created_at, organization_id FROM projects WHERE id = ? AND organization_id = ?",
    )
    .bind(id)
    .bind(organization_id)
    .fetch_optional(pool)
    .await
}

/// Find a project by ID.
pub async fn find_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT id, title, user_id, created_at, organization_id FROM projects WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
