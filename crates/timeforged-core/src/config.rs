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
