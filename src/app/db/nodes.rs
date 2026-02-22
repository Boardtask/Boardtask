use sqlx::FromRow;
use time::OffsetDateTime;

/// Database row for nodes table.
#[derive(Clone, Debug, FromRow, serde::Serialize)]
pub struct Node {
    pub id: String,
    pub project_id: String,
    pub node_type_id: String,
    pub status_id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub estimated_minutes: Option<i64>,
    pub slot_id: Option<String>,
    pub parent_id: Option<String>,
}

/// Data structure for inserting a new node.
pub struct NewNode {
    pub id: String,
    pub project_id: String,
    pub node_type_id: String,
    pub status_id: String,
    pub title: String,
    pub description: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub slot_id: Option<String>,
    pub parent_id: Option<String>,
}

/// Insert a new node into the database.
pub async fn insert<'e, E>(
    executor: E,
    node: &NewNode,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "INSERT INTO nodes (id, project_id, node_type_id, status_id, title, description, created_at, estimated_minutes, slot_id, parent_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&node.id)
    .bind(&node.project_id)
    .bind(&node.node_type_id)
    .bind(&node.status_id)
    .bind(&node.title)
    .bind(&node.description)
    .bind(now)
    .bind(&node.estimated_minutes)
    .bind(&node.slot_id)
    .bind(&node.parent_id)
    .execute(executor)
    .await?;

    Ok(())
}

/// Count nodes in a project.
pub async fn count_by_project(
    pool: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM nodes WHERE project_id = ?")
        .bind(project_id)
        .fetch_one(pool)
        .await
}

/// Count nodes in a project with the given status_id.
pub async fn count_by_project_and_status(
    pool: &sqlx::SqlitePool,
    project_id: &str,
    status_id: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM nodes WHERE project_id = ? AND status_id = ?")
        .bind(project_id)
        .bind(status_id)
        .fetch_one(pool)
        .await
}

/// Find all nodes for a project.
pub async fn find_by_project(
    pool: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<Vec<Node>, sqlx::Error> {
    sqlx::query_as::<_, Node>(
        "SELECT id, project_id, node_type_id, status_id, title, description, created_at, updated_at, estimated_minutes, slot_id, parent_id FROM nodes WHERE project_id = ? ORDER BY created_at",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

/// Find a node by ID.
pub async fn find_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<Node>, sqlx::Error> {
    sqlx::query_as::<_, Node>(
        "SELECT id, project_id, node_type_id, status_id, title, description, created_at, updated_at, estimated_minutes, slot_id, parent_id FROM nodes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Update a node's title, description, node_type_id, status_id, estimated_minutes, slot_id, parent_id, and updated_at timestamp.
/// parent_id: None means set column to NULL (clear parent), Some(pid) means set to pid.
pub async fn update(
    pool: &sqlx::SqlitePool,
    id: &str,
    title: &str,
    description: Option<&str>,
    node_type_id: &str,
    status_id: &str,
    estimated_minutes: Option<i64>,
    slot_id: Option<&str>,
    parent_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "UPDATE nodes SET title = ?, description = ?, node_type_id = ?, status_id = ?, estimated_minutes = ?, slot_id = ?, parent_id = ?, updated_at = ? WHERE id = ?",
    )
    .bind(title)
    .bind(description)
    .bind(node_type_id)
    .bind(status_id)
    .bind(estimated_minutes)
    .bind(slot_id)
    .bind(parent_id)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete a node by ID.
pub async fn delete(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM nodes WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}