pub mod config;
pub mod handlers;
pub mod openapi;
pub mod routes;
pub mod schema;
pub mod state;

pub use config::ServerConfig;
pub use routes::create_router;
pub use state::AppState;

use anyhow::Result;
use std::net::SocketAddr;
use tokio::signal;
use tracing::{info, warn};

/// Start the web server with the given configuration
pub async fn start_server(port: u16, config_path: &str, ui_path: &str) -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = ServerConfig::load(config_path).await?;

    // Create application state
    let state = AppState::new(config).await?;

    // Create router with all routes
    let app = create_router(state, ui_path);

    // Configure server address
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!(
        "🚀 gRPC Client Web UI starting on http://localhost:{}",
        port
    );
    info!("📁 Serving UI from: {}", ui_path);
    info!("📋 Using config file: {}", config_path);

    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("🛑 Server shutdown complete");
    Ok(())
}

/// Wait for graceful shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            warn!("\n🛑 Received Ctrl+C signal, shutting down gracefully...");
        },
        _ = terminate => {
            warn!("\n🛑 Received terminate signal, shutting down gracefully...");
        },
    }
}
