use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::event::CreateKind;
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

/// Обходит `root` и его вложенные каталоги, отдавая список того, что стоит
/// watch'ить НЕрекурсивно по отдельности.
///
/// Не идём по симлинкам и не спускаемся в то, что `is_ignored_path` и так
/// вырезает из событий (node_modules, .git, target, venv, dist, build,
/// __pycache__ и т.п.) — теми же правилами, что фильтруют события, теперь
/// фильтруется и то, что вообще становится вотчем.
///
/// Раньше это отдавалось `notify` целиком через `RecursiveMode::Recursive`,
/// а внутри него — `WalkDir::follow_links(true)`. В этом воркспейсе под
/// `deepseek-harness/.../node_modules` есть симлинк-цикл (pnpm кладёт
/// пакеты друг в друга через симлинки), и такой обход не заканчивается:
/// единственный event-loop-поток `notify` вешается навечно внутри
/// `add_watch()`, а его собственные `watches`/`paths` (`HashMap<PathBuf, _>`)
/// растут без остановки на всё более длинных путях каждого витка цикла —
/// это и был весь OOM (см. заметку в памяти по инциденту 2026-09-13).
fn collect_watch_dirs(root: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!("failed to read {}: {e}", dir.display());
                continue;
            }
        };
        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if file_type.is_symlink() || !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            if is_ignored_path(&path) {
                continue;
            }
            stack.push(path);
        }
        result.push(dir);
    }
    result
}

/// Имя проекта — первый каталог внутри отслеживаемого корня.
///
/// Файл, лежащий прямо в корне, проектом не является: у него нет каталога, чьё
/// имя можно взять, и раньше проектом становился он сам — случайный скриншот
/// рядом с репозиториями попадал в отчёт наравне с ними. Такой файл
/// пропускается: он не принадлежит ни одному проекту.
fn resolve_project_name(watched_root: &Path, file_path: &Path) -> Option<String> {
    let relative = file_path.strip_prefix(watched_root).ok()?;
    let mut components = relative.components();
    let first_component = components.next()?;
    // Ни одного компонента после первого — значит первый и есть сам файл.
    components.next()?;
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
    let watch_requests_tx = watcher_control_tx.clone();
    let rt_handle = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let rt = rt_handle;
        let event_tx = event_tx_clone;
        let initial_dirs = dirs_for_thread;
        // Каталоги, на которые сейчас реально стоит inotify-вотч (по одному
        // на каждый, все NonRecursive — см. collect_watch_dirs). Нужен,
        // чтобы на Unwatch(root) снять все вложенные вотчи: notify сам не
        // каскадирует unwatch для NonRecursive-регистраций.
        let mut registered: HashSet<PathBuf> = HashSet::new();

        let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                // Новый (не игнорируемый) каталог — досконально его тоже
                // нужно повотчить: сами мы регистрируем NonRecursive,
                // поэтому notify не подхватит его вложенные подкаталоги
                // автоматически, как делал бы в RecursiveMode::Recursive.
                if matches!(event.kind, EventKind::Create(CreateKind::Folder)) {
                    for path in &event.paths {
                        if !is_ignored_path(path) {
                            let _ = watch_requests_tx.try_send(WatcherControlMsg::Watch(path.clone()));
                        }
                    }
                }
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    for path in event.paths {
                        // Каталоги сюда не ходят: create-на-папке уже обработан веткой
                        // watch_requests выше, а modify-на-папке (chmod, rename родителя)
                        // не является работой над кодом -- раньше такие события долетали
                        // до event_tx с entity = сам каталог проекта и накручивали ему
                        // часы (520 событий на "tg-mcp" за один день 2026-09-13, ноль
                        // из них -- прикосновение человека).
                        if !path.is_dir() {
                            let _ = event_tx.try_send(path);
                        }
                    }
                }
            }
        }).expect("failed to create file watcher");

        watcher.configure(Config::default()).ok();

        // Watch initial dirs — по одному не-игнорируемому подкаталогу за
        // раз (collect_watch_dirs), не отдавая обход целиком notify.
        for dir in &initial_dirs {
            if dir.exists() {
                let mut watched = 0usize;
                for sub in collect_watch_dirs(dir) {
                    match watcher.watch(&sub, RecursiveMode::NonRecursive) {
                        Ok(()) => {
                            registered.insert(sub);
                            watched += 1;
                        }
                        Err(e) => tracing::warn!("failed to watch {}: {e}", sub.display()),
                    }
                }
                tracing::info!("watching {} ({watched} subdirectories)", dir.display());
            }
        }

        // Process control messages
        loop {
            match rt.block_on(watcher_control_rx.recv()) {
                Some(WatcherControlMsg::Watch(dir)) => {
                    if dir.exists() {
                        let mut watched = 0usize;
                        for sub in collect_watch_dirs(&dir) {
                            if registered.contains(&sub) {
                                continue;
                            }
                            match watcher.watch(&sub, RecursiveMode::NonRecursive) {
                                Ok(()) => {
                                    registered.insert(sub);
                                    watched += 1;
                                }
                                Err(e) => tracing::warn!("failed to watch {}: {e}", sub.display()),
                            }
                        }
                        if watched > 0 {
                            tracing::info!("watching {} ({watched} new subdirectories)", dir.display());
                        }
                    }
                }
                Some(WatcherControlMsg::Unwatch(dir)) => {
                    let nested: Vec<PathBuf> = registered
                        .iter()
                        .filter(|p| p.starts_with(&dir))
                        .cloned()
                        .collect();
                    for sub in nested {
                        let _ = watcher.unwatch(&sub);
                        registered.remove(&sub);
                    }
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
    gethostname::gethostname().into_string().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_is_the_first_directory_under_the_root() {
        let root = Path::new("/home/u/work");
        assert_eq!(
            resolve_project_name(root, Path::new("/home/u/work/boostix/src/main.rs")),
            Some("boostix".into()),
        );
    }

    #[test]
    fn file_directly_in_the_root_is_not_a_project() {
        // Скриншот, положенный рядом с репозиториями, раньше становился
        // проектом и попадал в отчёт наравне с ними.
        let root = Path::new("/home/u/work");
        assert_eq!(
            resolve_project_name(root, Path::new("/home/u/work/dashboard.png")),
            None,
        );
    }

    #[test]
    fn dotted_directory_is_skipped() {
        let root = Path::new("/home/u/work");
        assert_eq!(
            resolve_project_name(root, Path::new("/home/u/work/.cache/x/y.rs")),
            None,
        );
    }

    #[test]
    fn path_outside_the_root_has_no_project() {
        let root = Path::new("/home/u/work");
        assert_eq!(
            resolve_project_name(root, Path::new("/etc/passwd")),
            None,
        );
    }
}
