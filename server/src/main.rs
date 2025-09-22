use anyhow::Result;
use clap::Parser;
use org_parser::Org;
use tokio::sync::mpsc;
use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api_error;
mod cli;
mod config;
mod file_resolver;
mod json_output;
mod mcp;
mod notification;
mod parse;
mod reminders;
mod utils;
mod watcher;
mod web;

use cli::{Cli, Commands};
use json_output::{JsonOutputConfig, parse_and_output_json};
use mcp::start_mcp_server;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    // Manual help handling as a workaround
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        Cli::print_help(None);
        return Ok(());
    }

    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("org-server 0.1.0");
        return Ok(());
    }

    // Handle help subcommand manually
    if args.len() > 1 && args[1] == "help" {
        let subcommand = if args.len() > 2 {
            Some(args[2].as_str())
        } else {
            None
        };
        Cli::print_help(subcommand);
        return Ok(());
    }

    // Handle subcommand help
    if args.len() > 2 && args[1] == "parse" && (args[2] == "--help" || args[2] == "-h") {
        Cli::print_help(Some("parse"));
        return Ok(());
    }

    if args.len() > 2 && args[1] == "server" && (args[2] == "--help" || args[2] == "-h") {
        Cli::print_help(Some("server"));
        return Ok(());
    }

    // Handle CLI parsing with proper error handling
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            // For other errors, let clap handle them
            err.exit();
        }
    };

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

            let file_resolver = Arc::new(file_resolver::FileResolver::new(&server_config));

            let mut senders: Vec<mpsc::Sender<Org>> = Vec::new();

            // reminder
            check_reminder(&server_config, &mut senders);

            watcher::watch_files(&server_config, senders);

            let _mcp_handle = start_mcp_server(
                &server_config.mcp_host,
                server_config.mcp_port,
                Arc::clone(&file_resolver),
                &server_config,
            )
            .await?;

            // Use the port from CLI args if provided, otherwise use config
            let server_port = if port == 3000 {
                u16::try_from(server_config.server_port).unwrap_or_else(|_| {
                    eprintln!(
                        "Warning: server_port {} is too large for u16, using default 3000",
                        server_config.server_port
                    );
                    3000
                })
            } else {
                port
            };
            web::run_server(server_port.into(), server_config, file_resolver).await?;
        }
        Commands::Help { subcommand } => {
            Cli::print_help(subcommand.as_deref());
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
