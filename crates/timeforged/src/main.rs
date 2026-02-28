mod app;
mod auth;
mod handlers;
mod service;
mod storage;
mod watcher;
mod web;

use std::path::PathBuf;
use std::sync::Arc;

use sqlx::sqlite::SqlitePoolOptions;
use tokio::sync::{mpsc, Mutex};
use tracing_subscriber::EnvFilter;

use timeforged_core::config::{AppConfig, WatchedRegistry, WatcherConfig};

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
    let admin_key = user_service::ensure_admin(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if let Some(ref api_key) = admin_key {
        println!("==============================================");
        println!("  TimeForged — first run setup");
        println!("  Created admin user with API key:");
        println!("  {api_key}");
        println!("  Save this key! It won't be shown again.");
        println!("==============================================");
    }

    // Load watched directories and watcher config
    let registry = WatchedRegistry::load();
    let watcher_config = WatcherConfig::default();
    let initial_dirs: Vec<PathBuf> = registry
        .list()
        .iter()
        .map(|d| PathBuf::from(&d.path))
        .collect();

    // Create watcher command channel
    let (watcher_tx, watcher_rx) = mpsc::channel(256);

    // Get first user ID for watcher events
    let watcher_user_id = user_service::get_first_user(&pool)
        .await
        .ok()
        .map(|u| u.id);

    // Spawn file watcher
    if let Some(user_id) = watcher_user_id {
        let watcher_pool = pool.clone();
        let watcher_cfg = watcher_config.clone();
        let dirs = initial_dirs.clone();
        tokio::spawn(async move {
            watcher::file_watcher::run(watcher_pool, user_id, watcher_cfg, dirs, watcher_rx).await;
        });

        // Spawn window tracker
        let tracker_pool = pool.clone();
        let tracker_cfg = watcher_config.clone();
        let watched_dirs = Arc::new(Mutex::new(initial_dirs));
        tokio::spawn(async move {
            watcher::window_tracker::run(tracker_pool, user_id, tracker_cfg, watched_dirs).await;
        });

        if !registry.list().is_empty() {
            tracing::info!(
                "file watcher started for {} directories",
                registry.list().len()
            );
        }
    } else {
        tracing::debug!("no users yet, file watcher will start after first user creation");
        // Still consume the receiver so it doesn't block
        tokio::spawn(async move {
            let mut rx = watcher_rx;
            while rx.recv().await.is_some() {}
        });
    }

    let bind_addr = config.bind_addr();
    let state = AppState {
        db: pool,
        config,
        watcher_tx,
    };
    let router = build_router(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("TimeForged daemon listening on {bind_addr}");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
