use std::env;

use async_trait::async_trait;
use rand::prelude::{IndexedRandom, SliceRandom};
use sqlx::SqlitePool;

use crate::app::db::{self, NewUser};
use crate::app::domain::{Email, HashedPassword, Password, UserId};
use crate::seeds::{Seed, SeedOutcome};

const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const DIGIT: &[u8] = b"0123456789";

fn random_password() -> String {
    let mut rng = rand::rng();
    let mut chars: Vec<char> = vec![
        *UPPER.choose(&mut rng).unwrap() as char,
        *LOWER.choose(&mut rng).unwrap() as char,
        *DIGIT.choose(&mut rng).unwrap() as char,
    ];
    let pool: Vec<u8> = UPPER.iter().chain(LOWER).chain(DIGIT).copied().collect();
    for _ in 0..12 {
        chars.push(*pool.choose(&mut rng).unwrap() as char);
    }
    chars.shuffle(&mut rng);
    chars.into_iter().collect()
}

pub struct DevAdminUser;

#[async_trait]
impl Seed for DevAdminUser {
    fn version(&self) -> i64 {
        20260214120000
    }

    fn description(&self) -> &str {
        "dev_admin_user"
    }

    async fn run(&self, pool: &SqlitePool) -> Result<SeedOutcome, sqlx::Error> {
        let email_str = match env::var("SEED_ADMIN_EMAIL") {
            Ok(s) if !s.trim().is_empty() => s.trim().to_lowercase(),
            _ => return Ok(SeedOutcome::Skipped),
        };
        let email = match Email::new(email_str) {
            Ok(e) => e,
            Err(_) => return Ok(SeedOutcome::Skipped),
        };
        if db::find_by_email(pool, &email).await?.is_some() {
            return Ok(SeedOutcome::Applied);
        }

        let password = Password::new(random_password())
            .expect("random password meets strength requirements");
        let password_hash =
            HashedPassword::from_password(&password).expect("password hashing must succeed");
        let user_id = UserId::new();

        // Create organization for admin
        let org_id = crate::app::domain::OrganizationId::new();
        let org = crate::app::db::organizations::NewOrganization {
            id: org_id.clone(),
            name: "Admin Org".to_string(),
        };
        crate::app::db::organizations::insert(pool, &org).await?;

        let new_user = NewUser {
            id: user_id.clone(),
            email: email.clone(),
            password_hash,
            organization_id: org_id.clone(),
        };
        db::users::insert(pool, &new_user).await?;

        crate::app::db::organizations::add_member(pool, &org_id, &user_id, crate::app::domain::OrganizationRole::Owner).await?;

        db::mark_verified(pool, &user_id).await?;

        eprintln!(
            "Created admin: {} / {}",
            email.as_str(),
            std::str::from_utf8(password.as_bytes()).unwrap_or("<utf8?>")
        );
        Ok(SeedOutcome::Applied)
    }
}
