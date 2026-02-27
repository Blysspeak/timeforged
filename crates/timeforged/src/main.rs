mod app;
mod auth;
mod handlers;
mod service;
mod storage;

use sqlx::sqlite::SqlitePoolOptions;
use tracing_subscriber::EnvFilter;

use timeforged_core::config::AppConfig;

use crate::app::{AppState, build_router};
use crate::service::user_service;
use crate::storage::sqlite::init_db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .init();

    // Ensure data directory exists for SQLite
    if let Some(path) = config.database_url.strip_prefix("sqlite:") {
        let db_path = path.split('?').next().unwrap_or(path);
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    init_db(&pool).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    // Create admin user if no users exist
    if let Some(api_key) = user_service::ensure_admin(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?
    {
        println!("==============================================");
        println!("  TimeForged — first run setup");
        println!("  Created admin user with API key:");
        println!("  {api_key}");
        println!("  Save this key! It won't be shown again.");
        println!("==============================================");
    }

    let bind_addr = config.bind_addr();
    let state = AppState { db: pool, config };
    let router = build_router(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("TimeForged daemon listening on {bind_addr}");
    axum::serve(listener, router).await?;

    Ok(())
}
