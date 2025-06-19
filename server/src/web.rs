use anyhow::Result;
use axum::{Router, routing::get};
use tracing::{info, warn};

pub async fn run_server(port: u32) -> Result<()> {
    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root));

    // Try to bind to the specified port, with fallback options
    let mut current_port = port;
    let max_attempts = 10;

    for attempt in 0..max_attempts {
        let addr = format!("0.0.0.0:{}", current_port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                if current_port != port {
                    warn!(
                        "Original port {} was in use, using port {} instead",
                        port, current_port
                    );
                }
                info!("Server starting on {}", addr);
                axum::serve(listener, app).await?;
                return Ok(());
            }
            Err(err) => {
                if attempt == max_attempts - 1 {
                    return Err(err.into());
                }
                warn!(
                    "Port {} is in use, trying port {}",
                    current_port,
                    current_port + 1
                );
                current_port += 1;
            }
        }
    }

    unreachable!()
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}
