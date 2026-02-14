use sqlx::FromRow;
use time::OffsetDateTime;

/// Database row for node_types table.
#[derive(Debug, FromRow)]
pub struct NodeType {
    pub id: String,
    pub user_id: Option<String>,
    pub name: String,
    pub color: String,
    pub created_at: i64,
}

/// Data structure for inserting a new node type.
pub struct NewNodeType {
    pub id: String,
    pub user_id: Option<String>,
    pub name: String,
    pub color: String,
}

/// Insert a new node type into the database.
pub async fn insert<'e, E>(
    executor: E,
    node_type: &NewNodeType,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();

    sqlx::query(
        "INSERT INTO node_types (id, user_id, name, color, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&node_type.id)
    .bind(&node_type.user_id)
    .bind(&node_type.name)
    .bind(&node_type.color)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(())
}

/// Get all system node types (where user_id IS NULL).
pub async fn get_all_systems(pool: &sqlx::SqlitePool) -> Result<Vec<NodeType>, sqlx::Error> {
    sqlx::query_as::<_, NodeType>(
        "SELECT id, user_id, name, color, created_at FROM node_types WHERE user_id IS NULL ORDER BY name",
    )
    .fetch_all(pool)
    .await
}

/// Find a node type by ID.
pub async fn find_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<NodeType>, sqlx::Error> {
    sqlx::query_as::<_, NodeType>(
        "SELECT id, user_id, name, color, created_at FROM node_types WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}