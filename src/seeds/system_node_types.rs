use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::app::features::graph;
use crate::seeds::{Seed, SeedOutcome};

pub struct SystemNodeTypes;

#[async_trait]
impl Seed for SystemNodeTypes {
    fn version(&self) -> i64 {
        20260214115000
    }

    fn description(&self) -> &str {
        "system_node_types"
    }

    async fn run(&self, pool: &SqlitePool) -> Result<SeedOutcome, sqlx::Error> {
        graph::sync_system_node_types(pool).await?;
        Ok(SeedOutcome::Applied)
    }
}