use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use sqlx::SqlitePool;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use timeforged_core::config::WatcherConfig;
use timeforged_core::models::{ActivityType, EventType};
use timeforged_core::util::{infer_language_from_path, is_ignored_path};

use super::WatcherCommand;
use super::debounce::Debouncer;
use crate::storage::sqlite;

struct GitBranchCache {
    cache: HashMap<PathBuf, (String, Instant)>,
    ttl: Duration,
}

impl GitBranchCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            ttl: Duration::from_secs(60),
        }
    }

    async fn get_branch(&mut self, dir: &Path) -> Option<String> {
        if let Some((branch, when)) = self.cache.get(dir) {
            if when.elapsed() < self.ttl {
                return Some(branch.clone());
            }
        }

        let dir_owned = dir.to_path_buf();
        let result = tokio::task::spawn_blocking(move || {
            std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&dir_owned)
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                })
        })
        .await
        .ok()
        .flatten();

        if let Some(ref branch) = result {
            self.cache.insert(dir.to_path_buf(), (branch.clone(), Instant::now()));
        }
        result
    }
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

pub async fn run(
    pool: SqlitePool,
    user_id: Uuid,
    watcher_config: WatcherConfig,
    initial_dirs: Vec<PathBuf>,
    mut cmd_rx: mpsc::Receiver<WatcherCommand>,
) {
    let debouncer = Arc::new(Mutex::new(Debouncer::new(watcher_config.debounce_secs)));
    let git_cache = Arc::new(Mutex::new(GitBranchCache::new()));

    let (event_tx, mut event_rx) = mpsc::channel::<PathBuf>(1024);

    // Spawn the notify watcher in a blocking-friendly way
    let (watcher_control_tx, mut watcher_control_rx) = mpsc::channel::<WatcherControlMsg>(64);

    // Track watched roots for project resolution
    let watched_roots: Arc<Mutex<Vec<PathBuf>>> =
        Arc::new(Mutex::new(initial_dirs.clone()));

    // Spawn blocking watcher thread
    let event_tx_clone = event_tx.clone();
    let dirs_for_thread = initial_dirs;
    let rt_handle = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let rt = rt_handle;
        let event_tx = event_tx_clone;
        let initial_dirs = dirs_for_thread;

        let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    for path in event.paths {
                        let _ = event_tx.try_send(path);
                    }
                }
            }
        }).expect("failed to create file watcher");

        watcher.configure(Config::default()).ok();

        // Watch initial dirs
        for dir in &initial_dirs {
            if dir.exists() {
                if let Err(e) = watcher.watch(dir, RecursiveMode::Recursive) {
                    tracing::warn!("failed to watch {}: {e}", dir.display());
                } else {
                    tracing::info!("watching {}", dir.display());
                }
            }
        }

        // Process control messages
        loop {
            match rt.block_on(watcher_control_rx.recv()) {
                Some(WatcherControlMsg::Watch(dir)) => {
                    if dir.exists() {
                        if let Err(e) = watcher.watch(&dir, RecursiveMode::Recursive) {
                            tracing::warn!("failed to watch {}: {e}", dir.display());
                        } else {
                            tracing::info!("watching {}", dir.display());
                        }
                    }
                }
                Some(WatcherControlMsg::Unwatch(dir)) => {
                    let _ = watcher.unwatch(&dir);
                    tracing::info!("unwatched {}", dir.display());
                }
                None => break,
            }
        }
    });

    // Forward WatcherCommands to the watcher thread
    let watcher_control_tx_clone = watcher_control_tx.clone();
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            let msg = match cmd {
                WatcherCommand::Watch(p) => WatcherControlMsg::Watch(p),
                WatcherCommand::Unwatch(p) => WatcherControlMsg::Unwatch(p),
            };
            let _ = watcher_control_tx_clone.send(msg).await;
        }
    });

    // Process file events
    let mut cleanup_interval = tokio::time::interval(Duration::from_secs(300));

    loop {
        tokio::select! {
            Some(path) = event_rx.recv() => {
                if is_ignored_path(&path) {
                    continue;
                }

                // Debounce
                let should_emit = {
                    let mut db = debouncer.lock().await;
                    db.should_emit(&path)
                };
                if !should_emit {
                    continue;
                }

                // Resolve project from watched roots
                let project = {
                    let roots = watched_roots.lock().await;
                    roots.iter().find_map(|root| resolve_project_name(root, &path))
                };

                if project.is_none() {
                    continue;
                }

                let language = infer_language_from_path(path.to_str().unwrap_or(""));
                let entity = path.to_string_lossy().to_string();

                // Get git branch
                let branch = {
                    let mut cache = git_cache.lock().await;
                    // Find the project dir for git
                    let roots = watched_roots.lock().await;
                    let project_dir = roots.iter().find_map(|root| {
                        let rel = path.strip_prefix(root).ok()?;
                        let first = rel.components().next()?;
                        Some(root.join(first))
                    });
                    drop(roots);
                    if let Some(dir) = project_dir {
                        cache.get_branch(&dir).await
                    } else {
                        None
                    }
                };

                let machine = hostname();

                let event = timeforged_core::models::Event {
                    id: None,
                    user_id,
                    timestamp: chrono::Utc::now(),
                    event_type: EventType::File,
                    entity,
                    project,
                    language,
                    branch,
                    activity: Some(ActivityType::Coding),
                    machine,
                    metadata: None,
                    created_at: None,
                };

                if let Err(e) = sqlite::insert_event(&pool, &event).await {
                    tracing::warn!("failed to insert watcher event: {e}");
                }
            }
            _ = cleanup_interval.tick() => {
                let mut db = debouncer.lock().await;
                db.cleanup();
            }
        }
    }
}

enum WatcherControlMsg {
    Watch(PathBuf),
    Unwatch(PathBuf),
}

fn hostname() -> Option<String> {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .ok()
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok().map(|s| s.trim().to_string()))
}
