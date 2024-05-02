use anyhow::Result;
use clap::Parser;
use org_parser::Org;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod notification;
mod parse;
mod reminders;
mod utils;
mod watcher;
mod web;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct App {
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let app = App::parse();
    let config_path = if let Some(path) = app.config.as_deref() {
        PathBuf::from(path)
    } else {
        utils::get_config_file("org-server.toml")?
    };

    debug!("load config path: {:?}", config_path);
    let config = config::parse_config(&config_path.to_string_lossy())?;

    let mut senders: Vec<mpsc::Sender<Org>> = Vec::new();

    // reminder
    check_reminder(&config, &mut senders).await?;

    watcher::watch_files(&config, senders)?;

    web::run_server(config.server_port).await?;
    Ok(())
}

async fn check_reminder(
    config: &config::Config,
    senders: &mut Vec<mpsc::Sender<Org>>,
) -> Result<()> {
    let (tx, rx) = mpsc::channel(1024);
    senders.push(tx.clone());
    // start reminder checker
    reminders::start_check(rx).await?;
    reminders::scan(config, tx.clone())?;
    Ok(())
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
