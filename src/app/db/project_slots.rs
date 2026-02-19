use sqlx::FromRow;
use time::OffsetDateTime;

/// Database row for project_slots table.
#[derive(Debug, FromRow, serde::Serialize)]
pub struct ProjectSlot {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub sort_order: i64,
    pub created_at: i64,
}

/// Data structure for inserting a new project slot.
pub struct NewProjectSlot {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub sort_order: i64,
}

/// Insert a new project slot into the database.
pub async fn insert<'e, E>(
    executor: E,
    slot: &NewProjectSlot,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "INSERT INTO project_slots (id, project_id, name, sort_order, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&slot.id)
    .bind(&slot.project_id)
    .bind(&slot.name)
    .bind(slot.sort_order)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(())
}

/// Find all slots for a project, ordered by sort_order then name.
pub async fn find_by_project(
    pool: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<Vec<ProjectSlot>, sqlx::Error> {
    sqlx::query_as::<_, ProjectSlot>(
        "SELECT id, project_id, name, sort_order, created_at FROM project_slots WHERE project_id = ? ORDER BY sort_order, name",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

/// Find a project slot by ID.
pub async fn find_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<ProjectSlot>, sqlx::Error> {
    sqlx::query_as::<_, ProjectSlot>(
        "SELECT id, project_id, name, sort_order, created_at FROM project_slots WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Update a project slot's name and/or sort_order.
pub async fn update(
    pool: &sqlx::SqlitePool,
    id: &str,
    name: &str,
    sort_order: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE project_slots SET name = ?, sort_order = ? WHERE id = ?",
    )
    .bind(name)
    .bind(sort_order)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete a project slot by ID. Nodes referencing it will have slot_id set to NULL (ON DELETE SET NULL).
pub async fn delete(pool: &sqlx::SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM project_slots WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}
