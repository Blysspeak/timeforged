use std::path::Path;

use colored::Colorize;

use timeforged_core::api::{WatchActionResponse, WatchRequest};

use crate::client::TfClient;

pub async fn run(client: &TfClient, path: &str) {
    let canonical = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} invalid path: {e}", "error:".red().bold());
            std::process::exit(1);
        }
    };

    let path_str = canonical.to_string_lossy().to_string();
    let req = WatchRequest {
        path: path_str.clone(),
    };

    match client
        .post::<WatchActionResponse, _>("/api/v1/watch", &req)
        .await
    {
        Ok(resp) => {
            let already = resp.message.contains("already");

            if already {
                println!(
                    "{} {} is already being tracked",
                    "●".yellow().bold(),
                    path_str.bold()
                );
            } else {
                println!(
                    "{} Tracking enabled for {}",
                    "✓".green().bold(),
                    path_str.bold()
                );
            }

            // Discover projects (first-level subdirs)
            let projects = discover_projects(&canonical);
            if !projects.is_empty() {
                println!();
                println!(
                    "  {} projects found:",
                    projects.len().to_string().bold()
                );
                for name in &projects {
                    println!("    {} {}", "→".dimmed(), name);
                }
            }

            println!();
            println!(
                "  File changes will be tracked automatically."
            );
            println!(
                "  Run {} to see today's progress.",
                "tf today".green().bold()
            );
        }
        Err(e) => {
            if e.contains("connection refused") || e.contains("request failed") {
                eprintln!(
                    "{} daemon is not running. Start it with: {}",
                    "warning:".yellow().bold(),
                    "timeforged".bold()
                );
            } else {
                eprintln!("{} {e}", "error:".red().bold());
            }
            std::process::exit(1);
        }
    }
}

fn discover_projects(dir: &Path) -> Vec<String> {
    let mut projects = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !name.starts_with('.') {
                        projects.push(name.to_string());
                    }
                }
            }
        }
    }
    projects.sort();
    projects
}
