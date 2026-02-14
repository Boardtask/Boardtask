use boardtask::app;
use dotenvy::dotenv;
use sqlx::sqlite::SqlitePoolOptions;
use std::env;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Load .env file (silently ignore if missing)
    dotenv().ok();

    // Initialise structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug,tower_http=debug", env!("CARGO_PKG_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config from environment
    let config = app::config::Config::from_env()
        .expect("Failed to load config (check DATABASE_URL and other env vars)");

    // Connect to SQLite
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Enable WAL mode and set busy timeout
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await
        .expect("Failed to set WAL mode");

    sqlx::query("PRAGMA busy_timeout=5000")
        .execute(&pool)
        .await
        .expect("Failed to set busy timeout");

    // Run embedded migrations on startup
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    // Build the mail adapter from config
    let mail = app::mail::from_config(&config)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to initialize mail adapter: {}", e);
            std::process::exit(1);
        });

    // Build the application state
    let state = app::AppState {
        db: pool,
        mail,
        config,
    };
    let router = boardtask::create_router(state);

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");

    tracing::info!("Listening on http://localhost:3000");

    axum::serve(listener, router).await.unwrap();
}
