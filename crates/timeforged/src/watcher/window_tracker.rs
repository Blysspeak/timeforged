use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::Mutex;
use uuid::Uuid;

use timeforged_core::config::WatcherConfig;
use timeforged_core::models::{ActivityType, EventType};
use timeforged_core::util::infer_language_from_path;

use crate::storage::sqlite;

pub async fn run(
    pool: SqlitePool,
    user_id: Uuid,
    config: WatcherConfig,
    watched_dirs: Arc<Mutex<Vec<PathBuf>>>,
) {
    if !config.enable_window_tracker {
        tracing::debug!("window tracker disabled");
        return;
    }

    let interval = Duration::from_secs(config.window_poll_secs);
    let mut ticker = tokio::time::interval(interval);

    loop {
        ticker.tick().await;

        let title = match get_active_window_title().await {
            Some(t) => t,
            None => continue,
        };

        // Try to extract a file path from the window title
        let file_path = match extract_file_path(&title) {
            Some(p) => p,
            None => continue,
        };

        // Check if file is in a watched directory
        let dirs = watched_dirs.lock().await;
        let matched_root = dirs.iter().find(|root| file_path.starts_with(root.as_path()));
        let root = match matched_root {
            Some(r) => r.clone(),
            None => continue,
        };
        drop(dirs);

        let project = resolve_project_name(&root, &file_path);
        if project.is_none() {
            continue;
        }

        let language = infer_language_from_path(file_path.to_str().unwrap_or(""));
        let entity = file_path.to_string_lossy().to_string();

        let machine = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("HOST"))
            .ok();

        let event = timeforged_core::models::Event {
            id: None,
            user_id,
            timestamp: chrono::Utc::now(),
            event_type: EventType::File,
            entity,
            project,
            language,
            branch: None,
            activity: Some(ActivityType::Coding),
            machine,
            metadata: None,
            created_at: None,
        };

        if let Err(e) = sqlite::insert_event(&pool, &event).await {
            tracing::warn!("window tracker: failed to insert event: {e}");
        }
    }
}

async fn get_active_window_title() -> Option<String> {
    // Try hyprctl first (Hyprland)
    let result = tokio::process::Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .await;

    if let Ok(output) = result {
        if output.status.success() {
            if let Ok(text) = String::from_utf8(output.stdout) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(title) = json.get("title").and_then(|v| v.as_str()) {
                        return Some(title.to_string());
                    }
                }
            }
        }
    }

    // Fallback to xdotool
    let result = tokio::process::Command::new("xdotool")
        .args(["getactivewindow", "getwindowname"])
        .output()
        .await;

    if let Ok(output) = result {
        if output.status.success() {
            return String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().to_string());
        }
    }

    None
}

fn extract_file_path(title: &str) -> Option<PathBuf> {
    // Common editor title patterns:
    // "file.rs - ProjectName - VSCode"
    // "file.rs (~/projects/foo) - NVIM"
    // "/home/user/projects/foo/file.rs - Editor"

    // Look for absolute paths
    for part in title.split(|c: char| c == ' ' || c == '\t' || c == '—' || c == '–') {
        let trimmed = part.trim();
        if trimmed.starts_with('/') && trimmed.len() > 1 {
            let path = Path::new(trimmed);
            if path.extension().is_some() {
                return Some(path.to_path_buf());
            }
        }
    }

    // Look for ~ paths
    if let Some(home) = std::env::var("HOME").ok() {
        for part in title.split(|c: char| c == ' ' || c == '\t' || c == '(' || c == ')') {
            let trimmed = part.trim();
            if trimmed.starts_with("~/") {
                let expanded = trimmed.replacen("~/", &format!("{home}/"), 1);
                let path = Path::new(&expanded);
                if path.extension().is_some() {
                    return Some(path.to_path_buf());
                }
            }
        }
    }

    None
}

fn resolve_project_name(watched_root: &Path, file_path: &Path) -> Option<String> {
    let relative = file_path.strip_prefix(watched_root).ok()?;
    let first_component = relative.components().next()?;
    let name = first_component.as_os_str().to_str()?;
    if name.starts_with('.') {
        return None;
    }
    Some(name.to_string())
}
