//! RUNE HTTP Server binary

use axum::{
    routing::{get, post},
    Router,
};
use rune_core::RUNEEngine;
use rune_server::{handlers, AppState};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize OpenTelemetry tracing
    let enable_otel = std::env::var("OTEL_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if enable_otel {
        rune_server::tracing::init_tracing_stack("rune-server")?;
        info!("OpenTelemetry tracing enabled");
    } else {
        // Fallback to simple console logging
        use tracing_subscriber::{EnvFilter, FmtSubscriber};
        let subscriber = FmtSubscriber::builder()
            .with_env_filter(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("info,rune=debug")),
            )
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
        info!("Console logging enabled (set OTEL_ENABLED=true for OpenTelemetry)");
    }

    info!("Starting RUNE HTTP Server v{}", env!("CARGO_PKG_VERSION"));

    // Initialize Prometheus metrics
    rune_server::metrics::init_prometheus()?;

    // Initialize metric descriptions
    rune_server::metrics::init_metrics();

    // Create RUNE engine
    let engine = Arc::new(RUNEEngine::new());

    // TODO: Load configuration from file or environment
    // engine.load_config("config.rune")?;

    // Create application state
    let debug = std::env::var("DEBUG").is_ok();
    let state = AppState::with_debug(engine, debug);

    // Build the application
    let app = Router::new()
        // Authorization endpoints
        .route("/v1/authorize", post(handlers::authorize))
        .route("/v1/authorize/batch", post(handlers::batch_authorize))
        // Health checks
        .route("/health/live", get(handlers::health_live))
        .route("/health/ready", get(handlers::health_ready))
        // Metrics
        .route("/metrics", get(handlers::metrics))
        // Add state
        .with_state(state)
        // Add middleware
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    // Get bind address from environment or use default
    let addr: SocketAddr = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()?;

    info!("Listening on {}", addr);

    // Create the server
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Run the server with graceful shutdown
    let server = axum::serve(listener, app);

    // Set up shutdown signal handler
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Received shutdown signal, shutting down gracefully...");
    };

    // Run server with graceful shutdown
    server
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    // Cleanup OpenTelemetry on shutdown
    if enable_otel {
        info!("Flushing OpenTelemetry traces...");
        rune_server::tracing::shutdown_telemetry();
    }

    info!("Server shutdown complete");
    Ok(())
}