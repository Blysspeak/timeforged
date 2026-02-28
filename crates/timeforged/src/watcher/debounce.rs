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
