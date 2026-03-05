use colored::Colorize;

use crate::client::TfClient;

pub async fn run(client: &TfClient, public: bool) {
    let body = serde_json::json!({ "public": public });

    match client
        .put::<serde_json::Value, _>("/api/v1/me/public-profile", &body)
        .await
    {
        Ok(resp) => {
            let is_public = resp["public_profile"].as_bool().unwrap_or(false);
            let card_url = resp["card_url"].as_str().unwrap_or("");

            if is_public {
                println!("{} public profile enabled", "✓".green());
                println!("Card URL: {}", card_url.cyan());
            } else {
                println!("{} public profile disabled", "✓".green());
            }
        }
        Err(e) => {
            eprintln!("{}: {e}", "error".red());
            std::process::exit(1);
        }
    }
}
