use colored::Colorize;

use timeforged_core::api::StatusResponse;

use crate::client::TfClient;

pub async fn run(client: &TfClient) {
    match client.get::<StatusResponse>("/api/v1/status").await {
        Ok(s) => {
            println!("{}", "TimeForged Status".bold());
            println!("  Version:  {}", s.version);
            println!("  Status:   {}", s.status.green());
            println!("  Users:    {}", s.user_count);
            println!("  Events:   {}", s.event_count);
        }
        Err(e) => {
            // Try health endpoint as fallback
            match client.health().await {
                Ok(h) => {
                    println!("{}", "TimeForged Status".bold());
                    println!("  Version:  {}", h.version);
                    println!("  Status:   {}", h.status.green());
                    println!("  (authenticate with API key for full status)");
                }
                Err(_) => {
                    eprintln!("{}: {e}", "error".red());
                    eprintln!("Is the TimeForged daemon running?");
                    std::process::exit(1);
                }
            }
        }
    }
}
