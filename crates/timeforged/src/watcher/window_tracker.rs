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

        let machine = gethostname::gethostname().into_string().ok();

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

// --- Platform-specific: get active window title ---

#[cfg(unix)]
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

#[cfg(windows)]
async fn get_active_window_title() -> Option<String> {
    // Spawn blocking because Win32 API calls are synchronous
    tokio::task::spawn_blocking(|| {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        unsafe {
            let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }

            let mut buf = [0u16; 512];
            let len = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW(
                hwnd,
                buf.as_mut_ptr(),
                buf.len() as i32,
            );
            if len <= 0 {
                return None;
            }

            let title = OsString::from_wide(&buf[..len as usize]);
            title.into_string().ok()
        }
    })
    .await
    .ok()
    .flatten()
}

#[cfg(not(any(unix, windows)))]
async fn get_active_window_title() -> Option<String> {
    None
}

// --- Cross-platform: extract file path from window title ---

fn extract_file_path(title: &str) -> Option<PathBuf> {
    // Common editor title patterns:
    // "file.rs - ProjectName - VSCode"
    // "file.rs (~/projects/foo) - NVIM"
    // "/home/user/projects/foo/file.rs - Editor"
    // "C:\Users\user\projects\foo\file.rs - Editor"

    for part in title.split(|c: char| c == ' ' || c == '\t' || c == '\u{2014}' || c == '\u{2013}') {
        let trimmed = part.trim();

        // Unix absolute paths
        if trimmed.starts_with('/') && trimmed.len() > 1 {
            let path = Path::new(trimmed);
            if path.extension().is_some() {
                return Some(path.to_path_buf());
            }
        }

        // Windows absolute paths (C:\, D:\, etc.)
        if trimmed.len() > 3 && trimmed.as_bytes().get(1) == Some(&b':') {
            let third = trimmed.as_bytes().get(2);
            if third == Some(&b'\\') || third == Some(&b'/') {
                let path = Path::new(trimmed);
                if path.extension().is_some() {
                    return Some(path.to_path_buf());
                }
            }
        }
    }

    // Look for ~ paths (Unix-style home shortcut)
    if let Some(home) = dirs::home_dir() {
        for part in title.split(|c: char| c == ' ' || c == '\t' || c == '(' || c == ')') {
            let trimmed = part.trim();
            if trimmed.starts_with("~/") {
                let expanded: PathBuf = home.join(&trimmed[2..]);
                if expanded.extension().is_some() {
                    return Some(expanded);
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_file_path ──

    #[test]
    fn extract_unix_absolute_path() {
        let title = "/home/user/project/src/main.rs — Editor";
        let result = extract_file_path(title);
        assert_eq!(result, Some(PathBuf::from("/home/user/project/src/main.rs")));
    }

    #[test]
    fn extract_unix_path_vscode_style() {
        let title = "main.rs /home/user/project/src/main.rs VSCode";
        let result = extract_file_path(title);
        assert_eq!(result, Some(PathBuf::from("/home/user/project/src/main.rs")));
    }

    #[test]
    fn extract_tilde_path() {
        let title = "file.rs (~/projects/foo/src/file.rs) - NVIM";
        let result = extract_file_path(title);
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.ends_with("projects/foo/src/file.rs"));
        // Should be expanded (no ~)
        assert!(!path.to_string_lossy().contains('~'));
    }

    #[test]
    fn extract_no_path_returns_none() {
        assert_eq!(extract_file_path("Welcome - VSCode"), None);
        assert_eq!(extract_file_path("Untitled-1"), None);
        assert_eq!(extract_file_path(""), None);
    }

    #[test]
    fn extract_ignores_dirs_without_extension() {
        assert_eq!(extract_file_path("/home/user/project — Editor"), None);
    }

    // ── resolve_project_name ──

    #[test]
    fn resolve_project_from_watched_root() {
        let root = Path::new("/home/user/projects");
        let file = Path::new("/home/user/projects/myapp/src/main.rs");
        assert_eq!(resolve_project_name(root, file), Some("myapp".into()));
    }

    #[test]
    fn resolve_project_ignores_dotdirs() {
        let root = Path::new("/home/user/projects");
        let file = Path::new("/home/user/projects/.hidden/foo.rs");
        assert_eq!(resolve_project_name(root, file), None);
    }

    #[test]
    fn resolve_project_wrong_root_returns_none() {
        let root = Path::new("/home/user/projects");
        let file = Path::new("/home/other/file.rs");
        assert_eq!(resolve_project_name(root, file), None);
    }

    // ── hostname ──

    #[test]
    fn hostname_returns_value() {
        let machine = gethostname::gethostname().into_string().ok();
        assert!(machine.is_some(), "gethostname should return a hostname");
        assert!(!machine.unwrap().is_empty());
    }
}
