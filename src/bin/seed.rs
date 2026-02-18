use boardtask::app;
use boardtask::seeds;
use dotenvy::dotenv;
use sqlx::sqlite::SqlitePoolOptions;
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let config = app::config::Config::from_env()
        .expect("Failed to load config (check DATABASE_URL and other env vars)");

    let _db_lock = match app::single_writer::acquire(&config.database_url) {
        Ok(Some(guard)) => Some(guard),
        Ok(None) => None,
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await
        .expect("Failed to set WAL mode");

    sqlx::query("PRAGMA busy_timeout=5000")
        .execute(&pool)
        .await
        .expect("Failed to set busy timeout");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

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
    .execute(&pool)
    .await
    .expect("Failed to create _boardtask_seeds table");

    let args: Vec<String> = env::args().collect();
    let force_all = args.iter().any(|a| a == "--force-all");
    let force_version = args
        .iter()
        .position(|a| a == "--force")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<i64>().ok());

    let applied: Vec<i64> = sqlx::query_scalar::<_, i64>("SELECT version FROM _boardtask_seeds")
        .fetch_all(&pool)
        .await
        .expect("Failed to query applied seeds");

    for seed in seeds::all_seeds() {
        let version = seed.version();
        let description = seed.description();
        let already_applied = applied.contains(&version);
        let forced = force_all || force_version == Some(version);

        if !forced && already_applied {
            eprintln!("Skipping {} (already applied)", description);
            continue;
        }

        if forced && already_applied {
            sqlx::query("DELETE FROM _boardtask_seeds WHERE version = ?")
                .bind(version)
                .execute(&pool)
                .await
                .expect("Failed to remove seed from tracking for re-run");
        }

        eprintln!("Running {}...", description);
        let outcome = match seed.run(&pool).await {
            Ok(o) => o,
            Err(e) => {
                eprintln!("Seed {} failed: {}", description, e);
                std::process::exit(1);
            }
        };

        match outcome {
            seeds::SeedOutcome::Applied => {
                sqlx::query("INSERT INTO _boardtask_seeds (version, description) VALUES (?, ?)")
                    .bind(version)
                    .bind(description)
                    .execute(&pool)
                    .await
                    .expect("Failed to record seed success");
                eprintln!("Done {}", description);
            }
            seeds::SeedOutcome::Skipped => {
                eprintln!("Skipped {} (conditions not met, e.g. SEED_ADMIN_EMAIL unset)", description);
            }
        }
    }
}
