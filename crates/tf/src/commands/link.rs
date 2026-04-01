use colored::Colorize;

use timeforged_core::api::HealthResponse;
use timeforged_core::config::{CliConfig, config_dir};

use crate::client::TfClient;

pub async fn run(remote_key: &str, remote_url: &str) {
    // 1. Verify remote is reachable and the key works
    let remote_config = CliConfig {
        server_url: remote_url.to_string(),
        api_key: Some(remote_key.to_string()),
        remote_url: None,
        remote_key: None,
    };
    let remote = TfClient::new(&remote_config);

    print!("Checking remote {}... ", remote_url.cyan());
    match remote.get::<HealthResponse>("/health").await {
        Ok(resp) => {
            println!("{} (v{})", "ok".green(), resp.version);
        }
        Err(e) => {
            println!("{}", "failed".red());
            eprintln!("{}: could not reach remote: {e}", "error".red());
            std::process::exit(1);
        }
    }

    // 2. Update cli.toml
    let config_path = config_dir().join("cli.toml");
    let mut config = CliConfig::load();
    config.remote_url = Some(remote_url.to_string());
    config.remote_key = Some(remote_key.to_string());

    let content = format!(
        "server_url = \"{}\"\n{}\nremote_url = \"{}\"\nremote_key = \"{}\"\n",
        config.server_url,
        config
            .api_key
            .as_ref()
            .map(|k| format!("api_key = \"{}\"", k))
            .unwrap_or_default(),
        remote_url,
        remote_key,
    );

    std::fs::create_dir_all(config_dir()).ok();
    if let Err(e) = std::fs::write(&config_path, &content) {
        eprintln!("{}: failed to write config: {e}", "error".red());
        std::process::exit(1);
    }
    println!("{} config saved to {}", "✓".green(), config_path.display().to_string().dimmed());

    // 3. Run initial sync
    println!();
    println!("Running initial sync...");
    let local = TfClient::new(&config);
    let remote_for_sync = CliConfig {
        server_url: remote_url.to_string(),
        api_key: Some(remote_key.to_string()),
        remote_url: None,
        remote_key: None,
    };
    let remote_client = TfClient::new(&remote_for_sync);
    super::sync::run(&local, &remote_client).await;

    println!();
    println!(
        "{} This machine is now linked! Auto-sync will push events every 5 minutes.",
        "✓".green()
    );
}
