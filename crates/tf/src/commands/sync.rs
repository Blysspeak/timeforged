use colored::Colorize;

use timeforged_core::api::{BatchEventRequest, BatchEventResponse, CreateEventRequest, ExportEventsResponse, SyncStateFile};
use timeforged_core::config::config_dir;

use crate::client::TfClient;

fn sync_state_path() -> std::path::PathBuf {
    config_dir().join("sync-state.toml")
}

fn load_sync_state() -> SyncStateFile {
    let path = sync_state_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        toml::from_str(&content).unwrap_or(SyncStateFile {
            last_synced: None,
            events_synced: 0,
        })
    } else {
        SyncStateFile {
            last_synced: None,
            events_synced: 0,
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

pub async fn run(local: &TfClient, remote: &TfClient) {
    let mut sync_state = load_sync_state();

    // Build query params for local events export
    let mut query = vec![("limit", "5000".to_string())];
    if let Some(since) = &sync_state.last_synced {
        query.push(("since", since.to_rfc3339()));
    }

    let query_refs: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();

    // Fetch events from local daemon
    let export: ExportEventsResponse = match local
        .get_with_query("/api/v1/events", &query_refs)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: failed to fetch local events: {e}", "error".red());
            std::process::exit(1);
        }
    };

    if export.events.is_empty() {
        println!("{} no new events to sync", "✓".green());
        return;
    }

    println!(
        "Found {} new events to sync...",
        export.count.to_string().cyan()
    );

    // Convert to CreateEventRequests and send in batches of 100
    let batch_size = 100;
    let mut total_accepted = 0usize;
    let mut total_rejected = 0usize;
    let mut latest_timestamp = sync_state.last_synced;

    for chunk in export.events.chunks(batch_size) {
        let events: Vec<CreateEventRequest> = chunk
            .iter()
            .map(|e| {
                // Track the latest timestamp
                if latest_timestamp.is_none() || Some(e.timestamp) > latest_timestamp {
                    latest_timestamp = Some(e.timestamp);
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

        let batch = BatchEventRequest { events };

        match remote
            .post::<BatchEventResponse, _>("/api/v1/events/batch", &batch)
            .await
        {
            Ok(resp) => {
                total_accepted += resp.accepted;
                total_rejected += resp.rejected;
            }
            Err(e) => {
                eprintln!("{}: failed to push batch: {e}", "error".red());
                // Save what we've synced so far
                break;
            }
        }
    }

    // Update sync state
    sync_state.last_synced = latest_timestamp;
    sync_state.events_synced += total_accepted as u64;
    save_sync_state(&sync_state);

    println!(
        "{} synced {} events ({} rejected, {} total synced)",
        "✓".green(),
        total_accepted.to_string().cyan(),
        total_rejected,
        sync_state.events_synced,
    );
}
