use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{ActivityType, EventType};

// --- Event requests ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventRequest {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub entity: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub activity: Option<ActivityType>,
    #[serde(default)]
    pub machine: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEventRequest {
    pub events: Vec<CreateEventRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventResponse {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub entity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEventResponse {
    pub accepted: usize,
    pub rejected: usize,
}

// --- API key requests ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub id: uuid::Uuid,
    pub label: String,
    pub key: String,
}

// --- Generic responses ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub version: String,
    pub user_count: i64,
    pub event_count: i64,
}
