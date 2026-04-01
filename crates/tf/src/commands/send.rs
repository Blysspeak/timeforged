use chrono::Utc;
use colored::Colorize;

use timeforged_core::api::{CreateEventRequest, EventResponse};
use timeforged_core::models::EventType;

use crate::client::TfClient;

pub async fn run(
    client: &TfClient,
    entity: &str,
    project: Option<&str>,
    language: Option<&str>,
    event_type: Option<&str>,
) {
    let req = CreateEventRequest {
        timestamp: Utc::now(),
        event_type: event_type
            .map(EventType::from_str_lossy)
            .unwrap_or(EventType::File),
        entity: entity.to_string(),
        project: project.map(String::from),
        language: language.map(String::from),
        branch: None,
        activity: None,
        machine: hostname(),
        metadata: None,
    };

    match client.post::<EventResponse, _>("/api/v1/events", &req).await {
        Ok(resp) => {
            println!(
                "{} event #{} for {}",
                "Sent".green(),
                resp.id,
                resp.entity
            );
        }
        Err(e) => {
            eprintln!("{}: {e}", "error".red());
            std::process::exit(1);
        }
    }
}

fn hostname() -> Option<String> {
    gethostname::gethostname().into_string().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_returns_some() {
        let h = hostname();
        assert!(h.is_some());
        assert!(!h.unwrap().is_empty());
    }
}
