use sqlx::FromRow;
use time::OffsetDateTime;

/// Database row for task_statuses table.
#[derive(Debug, FromRow, serde::Serialize)]
pub struct TaskStatus {
    pub id: String,
    pub organization_id: Option<String>,
    pub name: String,
    pub sort_order: i64,
    pub created_at: i64,
}

/// Data structure for inserting a new task status.
pub struct NewTaskStatus {
    pub id: String,
    pub organization_id: Option<String>,
    pub name: String,
    pub sort_order: i64,
}

/// Insert a new task status into the database.
pub async fn insert<'e, E>(
    executor: E,
    task_status: &NewTaskStatus,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "INSERT INTO task_statuses (id, organization_id, name, sort_order, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&task_status.id)
    .bind(&task_status.organization_id)
    .bind(&task_status.name)
    .bind(task_status.sort_order)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(())
}

/// Get all task statuses (system only: organization_id IS NULL), ordered by sort_order.
pub async fn get_all_task_statuses(pool: &sqlx::SqlitePool) -> Result<Vec<TaskStatus>, sqlx::Error> {
    sqlx::query_as::<_, TaskStatus>(
        "SELECT id, organization_id, name, sort_order, created_at FROM task_statuses WHERE organization_id IS NULL ORDER BY sort_order",
    )
    .fetch_all(pool)
    .await
}

/// Find a task status by ID.
pub async fn find_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<TaskStatus>, sqlx::Error> {
    sqlx::query_as::<_, TaskStatus>(
        "SELECT id, organization_id, name, sort_order, created_at FROM task_statuses WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
