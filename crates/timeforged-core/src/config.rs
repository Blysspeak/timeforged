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
    #[serde(default = "default_sync_interval")]
    pub sync_interval: u64,
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

fn default_sync_interval() -> u64 {
    std::env::var("TF_SYNC_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
}

fn dirs_or_default() -> String {
    dirs_data().unwrap_or_else(|| {
        let tmp = std::env::temp_dir().join("timeforged");
        tmp.to_string_lossy().to_string()
    })
}

fn dirs_data() -> Option<String> {
    let base = dirs::data_dir()?;
    Some(base.join("timeforged").to_string_lossy().to_string())
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            database_url: default_database_url(),
            idle_timeout: default_idle_timeout(),
            log_level: default_log_level(),
            sync_interval: default_sync_interval(),
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
    #[serde(default)]
    pub remote_url: Option<String>,
    #[serde(default)]
    pub remote_key: Option<String>,
}

fn default_server_url() -> String {
    std::env::var("TF_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:6175".into())
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            api_key: std::env::var("TF_API_KEY").ok(),
            remote_url: std::env::var("TF_REMOTE_URL").ok(),
            remote_key: std::env::var("TF_REMOTE_KEY").ok(),
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
    dirs::config_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("timeforged")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_returns_timeforged_subdir() {
        let dir = config_dir();
        assert!(dir.ends_with("timeforged"), "config_dir should end with 'timeforged', got: {}", dir.display());
    }

    #[test]
    fn config_dir_is_absolute() {
        let dir = config_dir();
        assert!(dir.is_absolute(), "config_dir should be absolute, got: {}", dir.display());
    }

    #[test]
    fn app_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 6175);
        assert_eq!(config.idle_timeout, 300);
        assert_eq!(config.log_level, "info");
        assert_eq!(config.sync_interval, 300);
        assert!(config.database_url.starts_with("sqlite:"));
        assert!(config.database_url.contains("timeforged.db"));
    }

    #[test]
    fn app_config_bind_addr() {
        let config = AppConfig::default();
        assert_eq!(config.bind_addr(), "127.0.0.1:6175");
    }

    #[test]
    fn cli_config_defaults() {
        let config = CliConfig::default();
        assert_eq!(config.server_url, "http://127.0.0.1:6175");
    }

    #[test]
    fn app_config_from_toml() {
        let toml_str = r#"
            host = "0.0.0.0"
            port = 8080
            idle_timeout = 600
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.idle_timeout, 600);
        // Defaults for unset fields
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn cli_config_from_toml() {
        let toml_str = r#"
            server_url = "http://10.0.0.1:9000"
            api_key = "tf_testkey123"
            remote_url = "https://remote.example.com"
            remote_key = "tf_remotekey456"
        "#;
        let config: CliConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server_url, "http://10.0.0.1:9000");
        assert_eq!(config.api_key.unwrap(), "tf_testkey123");
        assert_eq!(config.remote_url.unwrap(), "https://remote.example.com");
        assert_eq!(config.remote_key.unwrap(), "tf_remotekey456");
    }

    #[test]
    fn watcher_config_defaults() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_secs, 30);
        assert_eq!(config.window_poll_secs, 15);
        assert!(!config.enable_window_tracker);
        assert!(config.ignore_patterns.is_empty());
    }

    #[test]
    fn watched_registry_add_remove() {
        let mut reg = WatchedRegistry::default();
        assert!(reg.list().is_empty());

        assert!(reg.add("/home/user/projects".into()));
        assert_eq!(reg.list().len(), 1);

        // Duplicate
        assert!(!reg.add("/home/user/projects".into()));
        assert_eq!(reg.list().len(), 1);

        // Second path
        assert!(reg.add("/home/user/work".into()));
        assert_eq!(reg.list().len(), 2);

        // Remove
        assert!(reg.remove("/home/user/projects"));
        assert_eq!(reg.list().len(), 1);

        // Remove non-existent
        assert!(!reg.remove("/nonexistent"));
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn database_url_contains_data_dir() {
        let url = default_database_url();
        assert!(url.starts_with("sqlite:"), "database_url should start with sqlite:");
        assert!(url.contains("timeforged"), "database_url should contain 'timeforged'");
    }
}
