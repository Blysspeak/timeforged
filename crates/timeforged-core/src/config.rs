use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_database_url")]
    pub database_url: String,
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: u64,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_host() -> String {
    std::env::var("TF_HOST").unwrap_or_else(|_| "127.0.0.1".into())
}

fn default_port() -> u16 {
    std::env::var("TF_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6175)
}

fn default_database_url() -> String {
    std::env::var("TF_DATABASE_URL").unwrap_or_else(|_| {
        let dir = dirs_or_default();
        format!("sqlite:{dir}/timeforged.db?mode=rwc")
    })
}

fn default_idle_timeout() -> u64 {
    std::env::var("TF_IDLE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
}

fn default_log_level() -> String {
    std::env::var("TF_LOG_LEVEL").unwrap_or_else(|_| "info".into())
}

fn dirs_or_default() -> String {
    dirs_data().unwrap_or_else(|| "/tmp/timeforged".into())
}

fn dirs_data() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    Some(format!("{home}/.local/share/timeforged"))
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            database_url: default_database_url(),
            idle_timeout: default_idle_timeout(),
            log_level: default_log_level(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let config_path = config_dir().join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    #[serde(default = "default_server_url")]
    pub server_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_server_url() -> String {
    std::env::var("TF_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:6175".into())
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            api_key: std::env::var("TF_API_KEY").ok(),
        }
    }
}

impl CliConfig {
    pub fn load() -> Self {
        let config_path = config_dir().join("cli.toml");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }
}

pub fn config_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(format!("{home}/.config/timeforged"))
}

// --- Watcher config ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    #[serde(default = "default_debounce_secs")]
    pub debounce_secs: u64,
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    #[serde(default = "default_window_poll_secs")]
    pub window_poll_secs: u64,
    #[serde(default)]
    pub enable_window_tracker: bool,
}

fn default_debounce_secs() -> u64 {
    30
}

fn default_window_poll_secs() -> u64 {
    15
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_secs: default_debounce_secs(),
            ignore_patterns: Vec::new(),
            window_poll_secs: default_window_poll_secs(),
            enable_window_tracker: false,
        }
    }
}

// --- Watched directory registry ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedDir {
    pub path: String,
    #[serde(default = "chrono::Utc::now")]
    pub added_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WatchedRegistry {
    #[serde(default)]
    pub dirs: Vec<WatchedDir>,
}

impl WatchedRegistry {
    pub fn load() -> Self {
        let path = config_dir().join("watched.toml");
        if let Ok(content) = std::fs::read_to_string(&path) {
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;
        std::fs::write(dir.join("watched.toml"), content)
    }

    pub fn add(&mut self, path: String) -> bool {
        if self.dirs.iter().any(|d| d.path == path) {
            return false;
        }
        self.dirs.push(WatchedDir {
            path,
            added_at: chrono::Utc::now(),
        });
        true
    }

    pub fn remove(&mut self, path: &str) -> bool {
        let len = self.dirs.len();
        self.dirs.retain(|d| d.path != path);
        self.dirs.len() < len
    }

    pub fn list(&self) -> &[WatchedDir] {
        &self.dirs
    }
}
