use sqlx::FromRow;
use time::OffsetDateTime;

/// Database row for nodes table.
#[derive(Debug, FromRow)]
pub struct Node {
    pub id: String,
    pub project_id: String,
    pub node_type_id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: Option<i64>,
}

/// Data structure for inserting a new node.
pub struct NewNode {
    pub id: String,
    pub project_id: String,
    pub node_type_id: String,
    pub title: String,
    pub description: Option<String>,
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
        "INSERT INTO nodes (id, project_id, node_type_id, title, description, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&node.id)
    .bind(&node.project_id)
    .bind(&node.node_type_id)
    .bind(&node.title)
    .bind(&node.description)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(())
}

/// Find all nodes for a project.
pub async fn find_by_project(
    pool: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<Vec<Node>, sqlx::Error> {
    sqlx::query_as::<_, Node>(
        "SELECT id, project_id, node_type_id, title, description, created_at, updated_at FROM nodes WHERE project_id = ? ORDER BY created_at",
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
        "SELECT id, project_id, node_type_id, title, description, created_at, updated_at FROM nodes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Update a node's title, description, and updated_at timestamp.
pub async fn update(
    pool: &sqlx::SqlitePool,
    id: &str,
    title: &str,
    description: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "UPDATE nodes SET title = ?, description = ?, updated_at = ? WHERE id = ?",
    )
    .bind(title)
    .bind(description)
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