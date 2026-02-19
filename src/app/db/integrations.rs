use sqlx::{FromRow, SqliteExecutor};
use time::OffsetDateTime;

/// Database row for integrations table (registry of allowed integrations).
#[derive(Debug, Clone, FromRow)]
pub struct Integration {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub created_at: i64,
}

/// Data structure for inserting a new integration.
pub struct NewIntegration {
    pub id: String,
    pub slug: String,
    pub name: String,
}

/// Insert a new integration.
pub async fn insert<'e, E>(
    executor: E,
    integration: &NewIntegration,
) -> Result<(), sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query(
        "INSERT INTO integrations (id, slug, name, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&integration.id)
    .bind(&integration.slug)
    .bind(&integration.name)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

/// List integration types for an organization (org-scoped; use this in app handlers).
/// Integration types come from the registry; _organization_id reserves per-org use (e.g. JOIN with organization_integrations later).
pub async fn list_for_org<'e, E>(
    executor: E,
    _organization_id: &str,
) -> Result<Vec<Integration>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, Integration>(
        "SELECT id, slug, name, created_at FROM integrations ORDER BY name",
    )
    .fetch_all(executor)
    .await
}

/// Find an integration by slug.
pub async fn find_by_slug<'e, E>(
    executor: E,
    slug: &str,
) -> Result<Option<Integration>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, Integration>(
        "SELECT id, slug, name, created_at FROM integrations WHERE slug = ?",
    )
    .bind(slug)
    .fetch_optional(executor)
    .await
}

// ---------------------------------------------------------------------------
// organization_integrations (per-org link, no settings column)
// ---------------------------------------------------------------------------

/// Database row for organization_integrations table.
#[derive(Debug, Clone, FromRow)]
pub struct OrganizationIntegration {
    pub organization_id: String,
    pub integration_id: String,
    pub enabled: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Find all org–integration links for an organization.
pub async fn find_org_integrations<'e, E>(
    executor: E,
    organization_id: &str,
) -> Result<Vec<OrganizationIntegration>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, OrganizationIntegration>(
        "SELECT organization_id, integration_id, enabled, created_at, updated_at FROM organization_integrations WHERE organization_id = ? ORDER BY integration_id",
    )
    .bind(organization_id)
    .fetch_all(executor)
    .await
}

/// Create or update an org–integration link. Sets enabled and updated_at.
pub async fn upsert_org_integration<'e, E>(
    executor: E,
    organization_id: &str,
    integration_id: &str,
    enabled: bool,
) -> Result<(), sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query(
        r#"INSERT INTO organization_integrations (organization_id, integration_id, enabled, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?)
           ON CONFLICT (organization_id, integration_id) DO UPDATE SET enabled = excluded.enabled, updated_at = excluded.updated_at"#,
    )
    .bind(organization_id)
    .bind(integration_id)
    .bind(if enabled { 1 } else { 0 })
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
