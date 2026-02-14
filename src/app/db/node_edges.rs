use sqlx::FromRow;
use time::OffsetDateTime;

/// Database row for node_edges table.
#[derive(Debug, FromRow)]
pub struct NodeEdge {
    pub parent_id: String,
    pub child_id: String,
    pub created_at: i64,
}

/// Data structure for inserting a new node edge.
pub struct NewNodeEdge {
    pub parent_id: String,
    pub child_id: String,
}

/// Insert a new node edge into the database.
pub async fn insert<'e, E>(
    executor: E,
    edge: &NewNodeEdge,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "INSERT INTO node_edges (parent_id, child_id, created_at) VALUES (?, ?, ?)",
    )
    .bind(&edge.parent_id)
    .bind(&edge.child_id)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(())
}

/// Delete a node edge by parent and child IDs.
pub async fn delete(
    pool: &sqlx::SqlitePool,
    parent_id: &str,
    child_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM node_edges WHERE parent_id = ? AND child_id = ?",
    )
    .bind(parent_id)
    .bind(child_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Find all child IDs for a given parent node.
pub async fn find_children_of(
    pool: &sqlx::SqlitePool,
    parent_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT child_id FROM node_edges WHERE parent_id = ? ORDER BY created_at",
    )
    .bind(parent_id)
    .fetch_all(pool)
    .await
}

/// Find all parent IDs for a given child node.
pub async fn find_parents_of(
    pool: &sqlx::SqlitePool,
    child_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT parent_id FROM node_edges WHERE child_id = ? ORDER BY created_at",
    )
    .bind(child_id)
    .fetch_all(pool)
    .await
}

/// Find all edges for a project (where both parent and child nodes belong to the project).
pub async fn find_by_project(
    pool: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<Vec<NodeEdge>, sqlx::Error> {
    sqlx::query_as::<_, NodeEdge>(
        "SELECT e.parent_id, e.child_id, e.created_at FROM node_edges e INNER JOIN nodes p ON e.parent_id = p.id INNER JOIN nodes c ON e.child_id = c.id WHERE p.project_id = ? AND c.project_id = ? ORDER BY e.created_at",
    )
    .bind(project_id)
    .bind(project_id)
    .fetch_all(pool)
    .await
}