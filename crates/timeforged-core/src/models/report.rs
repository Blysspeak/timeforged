use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub total_seconds: f64,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub projects: Vec<CategorySummary>,
    pub languages: Vec<CategorySummary>,
    pub days: Vec<DaySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySummary {
    pub name: String,
    pub total_seconds: f64,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySummary {
    pub date: NaiveDate,
    pub total_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_seconds: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub event_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyActivity {
    pub hour: u8,
    pub total_seconds: f64,
    pub event_count: i64,
}
