use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::app::db;
use crate::seeds::{Seed, SeedOutcome};

pub struct DevProjectSeed;

#[async_trait]
impl Seed for DevProjectSeed {
    fn version(&self) -> i64 {
        20260309000001
    }

    fn description(&self) -> &str {
        "dev_project"
    }

    async fn run(&self, pool: &SqlitePool) -> Result<SeedOutcome, sqlx::Error> {
        // Find the admin user
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

        // Check if project already exists
        let projects = db::projects::list_for_org(pool, &user.organization_id).await?;
        if !projects.is_empty() {
            return Ok(SeedOutcome::Applied);
        }

        // Find the team
        let teams = db::teams::find_by_organization(pool, &user.organization_id).await?;
        let team = match teams.first() {
            Some(t) => t,
            None => return Ok(SeedOutcome::Skipped),
        };

        // Create project
        let project_id = ulid::Ulid::new().to_string();
        let project = db::projects::NewProject {
            id: project_id.clone(),
            title: "Bug Reproduction Project".to_string(),
            user_id: user.id.clone(),
            organization_id: user.organization_id.clone(),
            team_id: team.id.clone(),
        };
        db::projects::insert(pool, &project).await?;

        // Create nodes
        let root_id = ulid::Ulid::new().to_string();
        let root_node = db::nodes::NewNode {
            id: root_id.clone(),
            project_id: project_id.clone(),
            node_type_id: "01JNODETYPE00000000TASK000".to_string(),
            status_id: "01JSTATUS00000000TODO0000".to_string(),
            title: "Root Task".to_string(),
            description: None,
            estimated_minutes: None,
            slot_id: None,
            parent_id: None,
            assigned_user_id: None,
        };
        db::nodes::insert(pool, &root_node).await?;

        let blocked_id = ulid::Ulid::new().to_string();
        let blocked_node = db::nodes::NewNode {
            id: blocked_id.clone(),
            project_id: project_id.clone(),
            node_type_id: "01JNODETYPE00000000TASK000".to_string(),
            status_id: "01JSTATUS00000000TODO0000".to_string(),
            title: "Blocked Task".to_string(),
            description: None,
            estimated_minutes: None,
            slot_id: None,
            parent_id: None,
            assigned_user_id: None,
        };
        db::nodes::insert(pool, &blocked_node).await?;

        // Create edge (Root -> Blocked)
        let edge = db::node_edges::NewNodeEdge {
            parent_id: root_id,
            child_id: blocked_id,
        };
        db::node_edges::insert(pool, &edge).await?;

        Ok(SeedOutcome::Applied)
    }
}
