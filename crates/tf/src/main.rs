mod client;
mod commands;

use clap::{Parser, Subcommand};

use timeforged_core::config::CliConfig;

use crate::client::TfClient;

#[derive(Parser)]
#[command(name = "tf", about = "TimeForged CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Server URL override
    #[arg(long, global = true)]
    server: Option<String>,

    /// API key override
    #[arg(long, global = true)]
    key: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show daemon status
    Status,
    /// Show today's summary
    Today,
    /// Generate a report
    Report {
        /// Time range: today, yesterday, week, month
        #[arg(long, default_value = "week")]
        range: String,
        /// Filter by project
        #[arg(long)]
        project: Option<String>,
        /// Custom start date (ISO 8601)
        #[arg(long)]
        from: Option<String>,
        /// Custom end date (ISO 8601)
        #[arg(long)]
        to: Option<String>,
    },
    /// Send a heartbeat event
    Send {
        /// Entity (file path, URL, etc.)
        entity: String,
        /// Project name
        #[arg(long)]
        project: Option<String>,
        /// Language
        #[arg(long)]
        language: Option<String>,
        /// Event type: file, terminal, browser, meeting, custom
        #[arg(long, name = "type")]
        event_type: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let mut config = CliConfig::load();

    if let Some(server) = cli.server {
        config.server_url = server;
    }
    if let Some(key) = cli.key {
        config.api_key = Some(key);
    }

    let client = TfClient::new(&config);

    match cli.command {
        Commands::Status => commands::status::run(&client).await,
        Commands::Today => commands::today::run(&client).await,
        Commands::Report {
            range,
            project,
            from,
            to,
        } => {
            let r = if from.is_some() || to.is_some() {
                commands::report::Range::Custom {
                    from: from.unwrap_or_default(),
                    to: to.unwrap_or_default(),
                }
            } else {
                match range.as_str() {
                    "today" => commands::report::Range::Today,
                    "yesterday" => commands::report::Range::Yesterday,
                    "week" => commands::report::Range::Week,
                    "month" => commands::report::Range::Month,
                    _ => {
                        eprintln!("Unknown range: {range}. Use today, yesterday, week, or month.");
                        std::process::exit(1);
                    }
                }
            };
            commands::report::run(&client, r, project.as_deref()).await;
        }
        Commands::Send {
            entity,
            project,
            language,
            event_type,
        } => {
            commands::send::run(
                &client,
                &entity,
                project.as_deref(),
                language.as_deref(),
                event_type.as_deref(),
            )
            .await;
        }
    }
}
