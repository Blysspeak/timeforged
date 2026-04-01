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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_roundtrip() {
        assert_eq!(EventType::from_str_lossy("file"), EventType::File);
        assert_eq!(EventType::from_str_lossy("terminal"), EventType::Terminal);
        assert_eq!(EventType::from_str_lossy("browser"), EventType::Browser);
        assert_eq!(EventType::from_str_lossy("meeting"), EventType::Meeting);
        assert_eq!(EventType::from_str_lossy("custom"), EventType::Custom);
        assert_eq!(EventType::from_str_lossy("unknown"), EventType::Custom);

        assert_eq!(EventType::File.as_str(), "file");
        assert_eq!(EventType::Terminal.as_str(), "terminal");
    }

    #[test]
    fn activity_type_roundtrip() {
        assert_eq!(ActivityType::from_str_lossy("coding"), ActivityType::Coding);
        assert_eq!(ActivityType::from_str_lossy("browsing"), ActivityType::Browsing);
        assert_eq!(ActivityType::from_str_lossy("unknown"), ActivityType::Other);

        assert_eq!(ActivityType::Coding.as_str(), "coding");
        assert_eq!(ActivityType::Other.as_str(), "other");
    }

    #[test]
    fn event_type_serde_json() {
        let event_type = EventType::File;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"file\"");

        let parsed: EventType = serde_json::from_str("\"terminal\"").unwrap();
        assert_eq!(parsed, EventType::Terminal);
    }

    #[test]
    fn activity_type_serde_json() {
        let activity = ActivityType::Coding;
        let json = serde_json::to_string(&activity).unwrap();
        assert_eq!(json, "\"coding\"");

        let parsed: ActivityType = serde_json::from_str("\"debugging\"").unwrap();
        assert_eq!(parsed, ActivityType::Debugging);
    }

    #[test]
    fn event_serialize_skip_none() {
        let event = Event {
            id: None,
            user_id: Uuid::nil(),
            timestamp: Utc::now(),
            event_type: EventType::File,
            entity: "test.rs".into(),
            project: None,
            language: Some("Rust".into()),
            branch: None,
            activity: None,
            machine: None,
            metadata: None,
            created_at: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("project"));
        assert!(!json.contains("branch"));
        assert!(json.contains("language"));
        assert!(json.contains("Rust"));
    }
}
