use boardtask::app;
use dotenvy::dotenv;
use sqlx::sqlite::SqlitePoolOptions;
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

    // Enforce single writer: one process per database file
    let _db_lock: Option<app::single_writer::SingleWriterGuard> = match app::single_writer::acquire(&config.database_url) {
        Ok(Some(guard)) => Some(guard),
        Ok(None) => None,
        Err(msg) => {
            tracing::error!("{}", msg);
            std::process::exit(1);
        }
    };

    // Connect to SQLite
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Production-safe SQLite: WAL protects main DB from mid-write damage; NORMAL balances safety and perf
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await
        .expect("Failed to set WAL mode");
    sqlx::query("PRAGMA synchronous=NORMAL")
        .execute(&pool)
        .await
        .expect("Failed to set synchronous");
    sqlx::query("PRAGMA busy_timeout=5000")
        .execute(&pool)
        .await
        .expect("Failed to set busy timeout");
    sqlx::query("PRAGMA foreign_keys=ON")
        .execute(&pool)
        .await
        .expect("Failed to enable foreign keys");

    // Run embedded migrations on startup
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    // Run seeds (system node types, etc.)
    boardtask::seeds::run_seeds(&pool)
        .await
        .expect("Failed to run seeds");

    // Build the mail adapter from config
    let mail = app::mail::from_config(&config)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to initialize mail adapter: {}", e);
            std::process::exit(1);
        });

    // Build the application state
    let state = app::AppState {
        db: pool.clone(),
        mail,
        config,
        resend_cooldown: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
    };
    let router = boardtask::create_router(state);

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");

    tracing::info!("Listening on http://localhost:3000");

    // Graceful shutdown: on SIGINT/SIGTERM stop accepting new requests, then close DB cleanly
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");

    // Close pool so SQLite checkpoints WAL and closes cleanly (prevents corruption)
    pool.close().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("Shutdown signal received, draining connections...");
}
