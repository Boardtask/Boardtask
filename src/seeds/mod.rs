mod dev_admin_user;
mod system_node_types;

use async_trait::async_trait;
use sqlx::SqlitePool;

/// Outcome of running a seed. Skipped seeds are not recorded so they may run again later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedOutcome {
    /// Seed executed and made changes; record in _boardtask_seeds.
    Applied,
    /// Seed chose not to run (e.g. env not set); do not record.
    Skipped,
}

/// A database seed. Seeds run in version order and are tracked for idempotency.
#[async_trait]
pub trait Seed: Send + Sync {
    /// Unique version identifier (timestamp format: YYYYMMDDHHMMSS).
    fn version(&self) -> i64;

    /// Human-readable description of the seed.
    fn description(&self) -> &str;

    /// Execute the seed. Uses the db layer; no raw SQL.
    /// Return Skipped when the seed opts out (e.g. missing env); it will not be recorded.
    async fn run(&self, pool: &SqlitePool) -> Result<SeedOutcome, sqlx::Error>;
}

/// All seeds in execution order (sorted by version).
pub fn all_seeds() -> Vec<Box<dyn Seed>> {
    let mut seeds: Vec<Box<dyn Seed>> = vec![
        Box::new(system_node_types::SystemNodeTypes),
        Box::new(dev_admin_user::DevAdminUser),
    ];
    seeds.sort_by_key(|s| s.version());
    seeds
}

/// Run all pending seeds using the given pool. Use this when the app is already
/// running so seeds share the app's connection pool instead of opening new ones.
pub async fn run_seeds(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    ensure_seeds_table(pool).await?;
    let applied = applied_versions(pool).await?;
    for seed in all_seeds() {
        let version = seed.version();
        let description = seed.description();
        if applied.contains(&version) {
            continue;
        }
        match seed.run(pool).await? {
            SeedOutcome::Applied => record_seed(pool, version, description).await?,
            SeedOutcome::Skipped => {}
        }
    }
    Ok(())
}

async fn ensure_seeds_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _boardtask_seeds (
            version INTEGER PRIMARY KEY NOT NULL,
            description TEXT NOT NULL,
            installed_on INTEGER NOT NULL DEFAULT (unixepoch()),
            success INTEGER NOT NULL DEFAULT 1
        )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn applied_versions(pool: &SqlitePool) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT version FROM _boardtask_seeds")
        .fetch_all(pool)
        .await
}

async fn record_seed(
    pool: &SqlitePool,
    version: i64,
    description: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO _boardtask_seeds (version, description) VALUES (?, ?)")
        .bind(version)
        .bind(description)
        .execute(pool)
        .await?;
    Ok(())
}
