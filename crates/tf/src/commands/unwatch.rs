use colored::Colorize;

use timeforged_core::api::{UnwatchRequest, WatchActionResponse};

use crate::client::TfClient;

pub async fn run(client: &TfClient, path: &str) {
    let canonical = std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string());

    let req = UnwatchRequest { path: canonical };

    match client
        .delete_with_body::<WatchActionResponse, _>("/api/v1/watch", &req)
        .await
    {
        Ok(resp) => {
            println!("{} {}", "✓".green().bold(), resp.message);
        }
        Err(e) => {
            eprintln!("{} {e}", "error:".red().bold());
            std::process::exit(1);
        }
    }
}
