use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::app::db;
use crate::seeds::{Seed, SeedOutcome};

pub struct DevTeamsSeed;

#[async_trait]
impl Seed for DevTeamsSeed {
    fn version(&self) -> i64 {
        20260309000000
    }

    fn description(&self) -> &str {
        "dev_teams"
    }

    async fn run(&self, pool: &SqlitePool) -> Result<SeedOutcome, sqlx::Error> {
        // Find the admin user created by dev_admin_user seed
        let admin_email = match std::env::var("SEED_ADMIN_EMAIL") {
            Ok(s) if !s.trim().is_empty() => s.trim().to_lowercase(),
            _ => return Ok(SeedOutcome::Skipped),
        };
        let email = match crate::app::domain::Email::new(admin_email) {
            Ok(e) => e,
            Err(_) => return Ok(SeedOutcome::Skipped),
        };
        
        let user = match db::find_by_email(pool, &email).await? {
            Some(u) => u,
            None => return Ok(SeedOutcome::Skipped),
        };

        // Check if the organization already has teams
        let existing_teams = db::teams::find_by_organization(pool, &user.organization_id).await?;
        if !existing_teams.is_empty() {
            return Ok(SeedOutcome::Applied);
        }

        // Create a default team for the admin's organization
        let team_id = ulid::Ulid::new().to_string();
        let new_team = db::teams::NewTeam {
            id: team_id,
            organization_id: user.organization_id.clone(),
            name: "Engineering".to_string(),
        };
        db::teams::insert(pool, &new_team).await?;

        Ok(SeedOutcome::Applied)
    }
}
