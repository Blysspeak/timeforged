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
    /// Start watching a directory for file changes
    Init {
        /// Directory to watch (default: current directory)
        path: Option<String>,
    },
    /// List watched directories
    List,
    /// Stop watching a directory
    Unwatch {
        /// Directory to stop watching
        path: String,
    },
    /// Set public profile visibility (enables card by username)
    Profile {
        /// Enable or disable public profile
        #[arg(long)]
        public: bool,
    },
    /// Sync local events to remote server
    Sync,
    /// Register on a remote TimeForged server
    Register {
        /// Username
        username: String,
        /// Display name
        #[arg(long)]
        display_name: Option<String>,
        /// Remote server URL (overrides config)
        #[arg(long)]
        remote: Option<String>,
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
        Commands::Init { path } => {
            let dir = path.unwrap_or_else(|| ".".to_string());
            commands::init::run(&client, &dir).await;
        }
        Commands::List => commands::list::run(&client).await,
        Commands::Unwatch { path } => {
            commands::unwatch::run(&client, &path).await;
        }
        Commands::Profile { public } => {
            commands::profile::run(&client, public).await;
        }
        Commands::Sync => {
            let remote_url = config.remote_url.clone().unwrap_or_else(|| {
                eprintln!("No remote URL configured. Set remote_url in ~/.config/timeforged/cli.toml or TF_REMOTE_URL env var.");
                std::process::exit(1);
            });
            let remote_config = CliConfig {
                server_url: remote_url,
                api_key: config.remote_key.clone(),
                remote_url: None,
                remote_key: None,
            };
            let remote = TfClient::new(&remote_config);
            commands::sync::run(&client, &remote).await;
        }
        Commands::Register {
            username,
            display_name,
            remote,
        } => {
            let remote_url = remote
                .or_else(|| config.remote_url.clone())
                .unwrap_or_else(|| {
                    eprintln!("No remote URL. Use --remote <url> or set remote_url in ~/.config/timeforged/cli.toml");
                    std::process::exit(1);
                });
            let remote_config = CliConfig {
                server_url: remote_url,
                api_key: None,
                remote_url: None,
                remote_key: None,
            };
            let remote_client = TfClient::new(&remote_config);
            commands::register::run(&remote_client, &username, display_name.as_deref()).await;
        }
    }
}
