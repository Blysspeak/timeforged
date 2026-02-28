use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL_CONDENSED};

use timeforged_core::api::WatchedResponse;

use crate::client::TfClient;

pub async fn run(client: &TfClient) {
    match client.get::<WatchedResponse>("/api/v1/watched").await {
        Ok(resp) => {
            if resp.dirs.is_empty() {
                println!("No directories are being watched.");
                println!(
                    "Use {} to start tracking.",
                    "tf init <path>".bold()
                );
                return;
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_header(vec!["Path", "Added"]);

            for dir in &resp.dirs {
                table.add_row(vec![
                    &dir.path,
                    &dir.added_at.format("%Y-%m-%d %H:%M").to_string(),
                ]);
            }

            println!("{table}");
        }
        Err(e) => {
            eprintln!("{} {e}", "error:".red().bold());
            std::process::exit(1);
        }
    }
}
