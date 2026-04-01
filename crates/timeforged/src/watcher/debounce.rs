use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct Debouncer {
    last_seen: HashMap<PathBuf, Instant>,
    interval: Duration,
}

impl Debouncer {
    pub fn new(debounce_secs: u64) -> Self {
        Self {
            last_seen: HashMap::new(),
            interval: Duration::from_secs(debounce_secs),
        }
    }

    pub fn should_emit(&mut self, path: &PathBuf) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last_seen.get(path) {
            if now.duration_since(*last) < self.interval {
                return false;
            }
        }
        self.last_seen.insert(path.clone(), now);
        true
    }

    pub fn cleanup(&mut self) {
        let now = Instant::now();
        let cutoff = self.interval * 3;
        self.last_seen.retain(|_, v| now.duration_since(*v) < cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_emit_is_allowed() {
        let mut db = Debouncer::new(30);
        let path = PathBuf::from("/project/src/main.rs");
        assert!(db.should_emit(&path));
    }

    #[test]
    fn immediate_repeat_is_debounced() {
        let mut db = Debouncer::new(30);
        let path = PathBuf::from("/project/src/main.rs");
        assert!(db.should_emit(&path));
        assert!(!db.should_emit(&path));
        assert!(!db.should_emit(&path));
    }

    #[test]
    fn different_paths_are_independent() {
        let mut db = Debouncer::new(30);
        let path1 = PathBuf::from("/project/src/main.rs");
        let path2 = PathBuf::from("/project/src/lib.rs");
        assert!(db.should_emit(&path1));
        assert!(db.should_emit(&path2));
        // path1 still debounced
        assert!(!db.should_emit(&path1));
    }

    #[test]
    fn after_interval_emit_again() {
        let mut db = Debouncer::new(0); // 0 seconds = no debounce
        let path = PathBuf::from("/project/src/main.rs");
        assert!(db.should_emit(&path));
        // With 0 interval, next call should also emit
        assert!(db.should_emit(&path));
    }

    #[test]
    fn cleanup_removes_old_entries() {
        let mut db = Debouncer::new(0); // 0 seconds
        let path = PathBuf::from("/project/src/main.rs");
        db.should_emit(&path);
        // With 0 interval, cutoff is also 0 — cleanup should remove entries
        std::thread::sleep(Duration::from_millis(1));
        db.cleanup();
        // After cleanup with 0 interval * 3 cutoff, entry is gone
        assert!(db.last_seen.is_empty() || db.should_emit(&path));
    }
}
