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

pub async fn run(local: &TfClient, remote: &TfClient) {
    let mut sync_state = load_sync_state();

    // Phase 1: Push (local → remote)
    println!("{}", "Pushing local → remote...".dimmed());
    let (pushed, push_rejected) = sync_events(local, remote, &mut sync_state, Direction::Push).await;

    // Phase 2: Pull (remote → local)
    println!("{}", "Pulling remote → local...".dimmed());
    let (pulled, pull_rejected) = sync_events(remote, local, &mut sync_state, Direction::Pull).await;

    // Summary
    if pushed == 0 && pulled == 0 && push_rejected == 0 && pull_rejected == 0 {
        println!("{} everything up to date", "✓".green());
    } else {
        if pushed > 0 || push_rejected > 0 {
            println!(
                "{} pushed {} events ({} rejected)",
                "↑".cyan(),
                pushed.to_string().cyan(),
                push_rejected,
            );
        }
        if pulled > 0 || pull_rejected > 0 {
            println!(
                "{} pulled {} events ({} rejected)",
                "↓".cyan(),
                pulled.to_string().cyan(),
                pull_rejected,
            );
        }
        println!(
            "  total synced: {} pushed, {} pulled",
            sync_state.events_synced,
            sync_state.events_pulled,
        );
    }
}

enum Direction {
    Push,
    Pull,
}

async fn sync_events(
    source: &TfClient,
    target: &TfClient,
    sync_state: &mut SyncStateFile,
    direction: Direction,
) -> (usize, usize) {
    let batch_size = 100;
    let page_size = 5_000;
    let mut grand_accepted = 0usize;
    let mut grand_rejected = 0usize;

    let last_timestamp = match direction {
        Direction::Push => &sync_state.last_synced,
        Direction::Pull => &sync_state.last_pulled,
    }
    .clone();

    let mut current_timestamp = last_timestamp;

    loop {
        let mut query = vec![("limit", page_size.to_string())];
        if let Some(since) = &current_timestamp {
            query.push(("since", since.to_rfc3339()));
        }

        let query_refs: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();

        let export: ExportEventsResponse = match source
            .get_with_query("/api/v1/events", &query_refs)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let label = match direction {
                    Direction::Push => "local",
                    Direction::Pull => "remote",
                };
                eprintln!("{}: failed to fetch {} events: {e}", "error".red(), label);
                break;
            }
        };

        if export.events.is_empty() {
            break;
        }

        let page_count = export.count;
        let arrow = match direction {
            Direction::Push => "↑",
            Direction::Pull => "↓",
        };
        println!(
            "  {} {} events to sync...",
            arrow,
            page_count.to_string().cyan()
        );

        let mut page_accepted = 0usize;
        let mut page_rejected = 0usize;
        let mut latest_timestamp = current_timestamp;
        let mut had_error = false;

        for chunk in export.events.chunks(batch_size) {
            let events: Vec<CreateEventRequest> = chunk
                .iter()
                .map(|e| {
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

            match target
                .post::<BatchEventResponse, _>("/api/v1/events/batch", &batch)
                .await
            {
                Ok(resp) => {
                    page_accepted += resp.accepted;
                    page_rejected += resp.rejected;
                }
                Err(e) => {
                    eprintln!("{}: failed to push batch: {e}", "error".red());
                    had_error = true;
                    break;
                }
            }
        }

        current_timestamp = latest_timestamp;

        match direction {
            Direction::Push => {
                sync_state.last_synced = current_timestamp;
                sync_state.events_synced += page_accepted as u64;
            }
            Direction::Pull => {
                sync_state.last_pulled = current_timestamp;
                sync_state.events_pulled += page_accepted as u64;
            }
        }
        save_sync_state(sync_state);

        grand_accepted += page_accepted;
        grand_rejected += page_rejected;

        if had_error || page_count < page_size {
            break;
        }
    }

    (grand_accepted, grand_rejected)
}
