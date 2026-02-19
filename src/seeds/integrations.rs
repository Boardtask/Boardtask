use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::app::db;
use crate::seeds::{Seed, SeedOutcome};

/// Default integrations to seed: (slug, display name).
const INTEGRATIONS: &[(&str, &str)] = &[
    ("github", "GitHub"),
    ("jira", "Jira"),
    ("gitlab", "GitLab"),
    ("bitbucket", "Bitbucket"),
];

pub struct IntegrationsSeed;

#[async_trait]
impl Seed for IntegrationsSeed {
    fn version(&self) -> i64 {
        20260219000000
    }

    fn description(&self) -> &str {
        "integrations"
    }

    async fn run(&self, pool: &SqlitePool) -> Result<SeedOutcome, sqlx::Error> {
        for (slug, name) in INTEGRATIONS {
            if db::integrations::find_by_slug(pool, slug).await?.is_some() {
                continue;
            }
            let integration = db::integrations::NewIntegration {
                id: ulid::Ulid::new().to_string(),
                slug: slug.to_string(),
                name: name.to_string(),
            };
            db::integrations::insert(pool, &integration).await?;
        }
        Ok(SeedOutcome::Applied)
    }
}
