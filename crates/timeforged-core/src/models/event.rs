use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    File,
    Terminal,
    Browser,
    Meeting,
    Custom,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Terminal => "terminal",
            Self::Browser => "browser",
            Self::Meeting => "meeting",
            Self::Custom => "custom",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "file" => Self::File,
            "terminal" => Self::Terminal,
            "browser" => Self::Browser,
            "meeting" => Self::Meeting,
            _ => Self::Custom,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    Coding,
    Browsing,
    Debugging,
    Building,
    Communicating,
    Designing,
    Other,
}

impl ActivityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Coding => "coding",
            Self::Browsing => "browsing",
            Self::Debugging => "debugging",
            Self::Building => "building",
            Self::Communicating => "communicating",
            Self::Designing => "designing",
            Self::Other => "other",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "coding" => Self::Coding,
            "browsing" => Self::Browsing,
            "debugging" => Self::Debugging,
            "building" => Self::Building,
            "communicating" => Self::Communicating,
            "designing" => Self::Designing,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<i64>,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub entity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<ActivityType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}
