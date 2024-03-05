use anyhow::Result;
use axum::{routing::get, Router};
use clap::Parser;
use std::path::PathBuf;

use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod notification;
mod parse;
mod remainders;
mod utils;
mod watcher;

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

    let (tx, rx) = tokio::sync::mpsc::channel(1024);

    watcher::watch_files(&config, tx.clone())?;

    // start checker
    remainders::start_check(rx).await?;
    remainders::scan(&config, tx.clone())?;

    run_server(config.server_port).await?;
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

async fn run_server(port: u32) -> Result<()> {
    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("start server");
    axum::serve(listener, app).await?;
    Ok(())
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}
