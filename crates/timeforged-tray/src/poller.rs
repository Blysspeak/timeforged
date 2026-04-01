use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;

const POLL_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize)]
pub(crate) struct SummaryResponse {
    #[serde(default)]
    pub total_seconds: f64,
    #[serde(default)]
    pub projects: Vec<ProjectSummary>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProjectSummary {
    pub name: String,
    #[serde(default)]
    pub total_seconds: f64,
}

pub async fn run(server_url: String, api_key: String, state: Arc<Mutex<String>>) {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    loop {
        let tooltip = fetch_tooltip(&client, &server_url, &api_key).await;
        if let Ok(mut s) = state.lock() {
            *s = tooltip;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn fetch_tooltip(client: &Client, server_url: &str, api_key: &str) -> String {
    let now = Utc::now();
    let today = now.format("%Y-%m-%dT00:00:00Z");
    let tomorrow = (now + chrono::Duration::days(1)).format("%Y-%m-%dT00:00:00Z");

    let url = format!(
        "{}/api/v1/reports/summary?from={}&to={}",
        server_url.trim_end_matches('/'),
        today,
        tomorrow
    );

    let result = client
        .get(&url)
        .header("X-Api-Key", api_key)
        .send()
        .await;

    let resp = match result {
        Ok(r) if r.status().is_success() => r,
        _ => return "TimeForged: offline".into(),
    };

    let summary: SummaryResponse = match resp.json().await {
        Ok(s) => s,
        Err(_) => return "TimeForged: error".into(),
    };

    format_tooltip(&summary)
}

pub(crate) fn format_tooltip(summary: &SummaryResponse) -> String {
    let hours = (summary.total_seconds / 3600.0) as u32;
    let mins = ((summary.total_seconds % 3600.0) / 60.0) as u32;

    let mut lines = vec![format!("TimeForged — {}:{:02} today", hours, mins)];

    // Per-project breakdown (skip < 1min)
    let mut projects: Vec<&ProjectSummary> = summary
        .projects
        .iter()
        .filter(|p| p.total_seconds >= 60.0)
        .collect();
    projects.sort_by(|a, b| b.total_seconds.partial_cmp(&a.total_seconds).unwrap());

    for p in projects.iter().take(8) {
        let ph = (p.total_seconds / 3600.0) as u32;
        let pm = ((p.total_seconds % 3600.0) / 60.0) as u32;
        // Take just the last path component as project name
        let name = p.name.rsplit('/').next().unwrap_or(&p.name);
        let name = name.rsplit('\\').next().unwrap_or(name);
        lines.push(format!("  {}:{:02}  {}", ph, pm, name));
    }

    if projects.is_empty() && summary.total_seconds < 60.0 {
        lines.push("  no activity yet".into());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_tooltip_no_activity() {
        let summary = SummaryResponse {
            total_seconds: 0.0,
            projects: vec![],
        };
        let result = format_tooltip(&summary);
        assert!(result.contains("0:00 today"));
        assert!(result.contains("no activity yet"));
    }

    #[test]
    fn format_tooltip_with_time() {
        let summary = SummaryResponse {
            total_seconds: 7380.0, // 2h 3m
            projects: vec![
                ProjectSummary { name: "myapp".into(), total_seconds: 5400.0 },    // 1h 30m
                ProjectSummary { name: "timeforged".into(), total_seconds: 1980.0 }, // 33m
            ],
        };
        let result = format_tooltip(&summary);
        assert!(result.contains("2:03 today"));
        assert!(result.contains("myapp"));
        assert!(result.contains("timeforged"));
        assert!(result.contains("1:30"));
        assert!(result.contains("0:33"));
    }

    #[test]
    fn format_tooltip_skips_under_1min() {
        let summary = SummaryResponse {
            total_seconds: 120.0,
            projects: vec![
                ProjectSummary { name: "active".into(), total_seconds: 100.0 },
                ProjectSummary { name: "tiny".into(), total_seconds: 20.0 }, // < 60s
            ],
        };
        let result = format_tooltip(&summary);
        assert!(result.contains("active"));
        assert!(!result.contains("tiny"));
    }

    #[test]
    fn format_tooltip_strips_path() {
        let summary = SummaryResponse {
            total_seconds: 3600.0,
            projects: vec![
                ProjectSummary { name: "/home/user/projects/myapp".into(), total_seconds: 3600.0 },
            ],
        };
        let result = format_tooltip(&summary);
        assert!(result.contains("myapp"));
        assert!(!result.contains("/home/user"));
    }

    #[test]
    fn format_tooltip_strips_windows_path() {
        let summary = SummaryResponse {
            total_seconds: 3600.0,
            projects: vec![
                ProjectSummary { name: "C:\\Users\\dev\\projects\\myapp".into(), total_seconds: 3600.0 },
            ],
        };
        let result = format_tooltip(&summary);
        assert!(result.contains("myapp"));
        assert!(!result.contains("C:\\"));
    }

    #[test]
    fn format_tooltip_max_8_projects() {
        let projects: Vec<ProjectSummary> = (0..12)
            .map(|i| ProjectSummary {
                name: format!("project{}", i),
                total_seconds: 120.0,
            })
            .collect();
        let summary = SummaryResponse {
            total_seconds: 1440.0,
            projects,
        };
        let result = format_tooltip(&summary);
        // Should only show 8 projects
        let project_lines: Vec<&str> = result.lines().filter(|l| l.starts_with("  ") && l.contains("project")).collect();
        assert_eq!(project_lines.len(), 8);
    }

    #[test]
    fn format_tooltip_sorts_by_time_desc() {
        let summary = SummaryResponse {
            total_seconds: 7200.0,
            projects: vec![
                ProjectSummary { name: "small".into(), total_seconds: 600.0 },
                ProjectSummary { name: "big".into(), total_seconds: 6000.0 },
                ProjectSummary { name: "medium".into(), total_seconds: 600.0 },
            ],
        };
        let result = format_tooltip(&summary);
        let big_pos = result.find("big").unwrap();
        let small_pos = result.find("small").unwrap();
        assert!(big_pos < small_pos, "big should come before small");
    }

    #[test]
    fn summary_response_deserialize() {
        let json = r#"{"total_seconds": 3661.0, "projects": [{"name": "test", "total_seconds": 3661.0}]}"#;
        let summary: SummaryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(summary.total_seconds, 3661.0);
        assert_eq!(summary.projects.len(), 1);
        assert_eq!(summary.projects[0].name, "test");
    }

    #[test]
    fn summary_response_deserialize_empty() {
        let json = r#"{}"#;
        let summary: SummaryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(summary.total_seconds, 0.0);
        assert!(summary.projects.is_empty());
    }
}
