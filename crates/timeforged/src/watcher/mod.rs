pub mod debounce;
pub mod file_watcher;
pub mod window_tracker;

use std::path::PathBuf;

#[derive(Debug)]
pub enum WatcherCommand {
    Watch(PathBuf),
    Unwatch(PathBuf),
}
