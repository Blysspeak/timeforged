use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::Client;
use sqlx::SqlitePool;
use uuid::Uuid;

use timeforged_core::api::{
    BatchEventRequest, BatchEventResponse, CreateEventRequest, SyncStateFile,
};
use timeforged_core::config::{CliConfig, config_dir};

use crate::storage::sqlite;

const BATCH_SIZE: usize = 100;
const PAGE_LIMIT: i64 = 5000;

fn sync_state_path() -> std::path::PathBuf {
    config_dir().join("sync-state.toml")
}

fn load_sync_state() -> SyncStateFile {
    let path = sync_state_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        toml::from_str(&content).unwrap_or(SyncStateFile {
            last_synced: None,
            events_synced: 0,
            last_pulled: None,
            events_pulled: 0,
        })
    } else {
        SyncStateFile {
            last_synced: None,
            events_synced: 0,
            last_pulled: None,
            events_pulled: 0,
        }
    }
}

fn save_sync_state(state: &SyncStateFile) {
    let path = sync_state_path();
    if let Ok(content) = toml::to_string_pretty(state) {
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let _ = std::fs::write(&path, content);
    }
}

async fn push_batch(
    http: &Client,
    remote_url: &str,
    remote_key: &str,
    batch: &BatchEventRequest,
) -> Result<BatchEventResponse, String> {
    let url = format!("{}/api/v1/events/batch", remote_url.trim_end_matches('/'));
    let resp = http
        .post(&url)
        .header("X-Api-Key", remote_key)
        .json(batch)
        .send()
        .await
        .map_err(|e| format!("remote request failed: {e}"))?;

    if resp.status().is_success() {
        resp.json::<BatchEventResponse>()
            .await
            .map_err(|e| format!("parse error: {e}"))
    } else {
        let status = resp.status();
        Err(format!("remote returned HTTP {status}"))
    }
}

async fn sync_once(pool: &SqlitePool, user_id: Uuid, http: &Client, remote_url: &str, remote_key: &str) {
    let mut state = load_sync_state();
    let since = state
        .last_synced
        .unwrap_or_else(|| DateTime::<Utc>::MIN_UTC);

    let events = match sqlite::list_events(pool, user_id, since, PAGE_LIMIT).await {
        Ok(ev) => ev,
        Err(e) => {
            tracing::warn!("auto-sync: failed to query events: {e}");
            return;
        }
    };

    if events.is_empty() {
        return;
    }

    let total = events.len();
    let mut accepted = 0usize;
    let mut latest_ts = state.last_synced;

    for chunk in events.chunks(BATCH_SIZE) {
        let batch_events: Vec<CreateEventRequest> = chunk
            .iter()
            .map(|e| {
                if latest_ts.is_none() || Some(e.timestamp) > latest_ts {
                    latest_ts = Some(e.timestamp);
                }
                CreateEventRequest {
                    timestamp: e.timestamp,
                    event_type: e.event_type.clone(),
                    entity: e.entity.clone(),
                    project: e.project.clone(),
                    language: e.language.clone(),
                    branch: e.branch.clone(),
                    activity: e.activity.clone(),
                    machine: e.machine.clone(),
                    metadata: e.metadata.clone(),
                }
            })
            .collect();

        match push_batch(http, remote_url, remote_key, &BatchEventRequest { events: batch_events }).await {
            Ok(resp) => accepted += resp.accepted,
            Err(e) => {
                tracing::warn!("auto-sync: batch push failed: {e}");
                break;
            }
        }
    }

    if accepted > 0 {
        state.last_synced = latest_ts;
        state.events_synced += accepted as u64;
        save_sync_state(&state);
        tracing::info!("auto-sync: pushed {accepted}/{total} events to remote");
    }
}

pub async fn run(pool: SqlitePool, user_id: Uuid, interval_secs: u64) {
    let cli_config = CliConfig::load();

    let (remote_url, remote_key) = match (cli_config.remote_url, cli_config.remote_key) {
        (Some(url), Some(key)) => (url, key),
        _ => {
            tracing::debug!("auto-sync: no remote configured, skipping");
            return;
        }
    };

    let http = Client::new();
    let interval = Duration::from_secs(interval_secs);

    // Initial delay to let the daemon fully start
    tokio::time::sleep(Duration::from_secs(10)).await;

    tracing::info!("auto-sync: started (interval={}s)", interval_secs);

    loop {
        sync_once(&pool, user_id, &http, &remote_url, &remote_key).await;
        tokio::time::sleep(interval).await;
    }
}
