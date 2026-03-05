use colored::Colorize;

use timeforged_core::api::{RegisterRequest, RegisterResponse};

use crate::client::TfClient;

pub async fn run(remote: &TfClient, username: &str, display_name: Option<&str>) {
    let req = RegisterRequest {
        username: username.to_string(),
        display_name: display_name.map(String::from),
    };

    match remote
        .post::<RegisterResponse, _>("/api/v1/register", &req)
        .await
    {
        Ok(resp) => {
            println!("{} registered as {}", "✓".green(), resp.username.cyan());
            println!();
            println!("Your API key: {}", resp.api_key.yellow());
            println!();
            println!("Add to {}:", "~/.config/timeforged/cli.toml".dimmed());
            println!("  remote_key = \"{}\"", resp.api_key);
            println!();
            println!(
                "Or export: {}",
                format!("export TF_REMOTE_KEY={}", resp.api_key).dimmed()
            );
        }
        Err(e) => {
            eprintln!("{}: {e}", "error".red());
            std::process::exit(1);
        }
    }
}
