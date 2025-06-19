use anyhow::Result;
use org_parser::Org;
use tokio::sync::mpsc;
use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod config;
mod json_output;
mod notification;
mod parse;
mod reminders;
mod utils;
mod watcher;
mod web;

use cli::{Cli, Commands};
use json_output::{JsonOutputConfig, parse_and_output_json};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse_args();
    
    match cli.get_command() {
        Commands::Parse {
            file,
            output,
            pretty,
            include_position,
            include_empty_sections,
            max_depth,
        } => {
            info!("Running in parse mode for file: {}", file.display());
            
            let json_config = JsonOutputConfig {
                pretty,
                include_position,
                include_empty_sections,
                max_depth,
            };

            parse_and_output_json(&file, output.as_ref(), json_config)?;
        }
        Commands::Server { config, port, host } => {
            info!("Running in server mode on {}:{}", host, port);
            
            let config_path = if let Some(path) = config {
                path
            } else {
                utils::get_config_file("org-server.toml")?
            };

            debug!("load config path: {:?}", config_path);
            let server_config = config::parse_config(&config_path.to_string_lossy())?;

            let mut senders: Vec<mpsc::Sender<Org>> = Vec::new();

            // reminder
            check_reminder(&server_config, &mut senders);

            watcher::watch_files(&server_config, senders);

            // Use the port from CLI args if provided, otherwise use config
            let server_port = if port == 3000 { 
                u16::try_from(server_config.server_port)
                    .unwrap_or_else(|_| {
                        eprintln!("Warning: server_port {} is too large for u16, using default 3000", server_config.server_port);
                        3000
                    })
            } else { 
                port 
            };
            web::run_server(server_port.into()).await?;
        }
    }
    
    Ok(())
}

fn check_reminder(config: &config::Config, senders: &mut Vec<mpsc::Sender<Org>>) {
    let (tx, rx) = mpsc::channel(1024);
    senders.push(tx.clone());
    // start reminder checker
    reminders::start_check(rx);
    reminders::scan(config, &tx);
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "org_server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
