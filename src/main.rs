use dotenvy::dotenv;
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use axum::{
    Router,
    routing::get,
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
    response::IntoResponse,
};
use tokio::{signal, time::interval};
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod service_islands;

use service_islands::ServiceIslands;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "web_server_report_websocket=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting WebSocket Service with Service Islands Architecture...");

    // Initialize Service Islands Architecture
    info!("🏝️ Initializing Service Islands Architecture...");
    let service_islands = Arc::new(ServiceIslands::initialize().await?);

    // Perform initial health check
    info!("🔍 Performing initial health check...");
    if service_islands.health_check().await {
        info!("✅ Service Islands Architecture is healthy!");
    } else {
        warn!("⚠️ Some Service Islands may have issues - continuing with startup...");
    }

    // Spawn background task for periodic market data fetching
    let islands_clone = service_islands.clone();
    tokio::spawn(async move {
        spawn_market_data_fetcher(islands_clone).await;
    });

    // Create router with WebSocket endpoint
    let app = create_router(service_islands.clone());

    // Start server
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse()
        .expect("PORT must be a valid number");

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("HOST and PORT must form a valid address");

    info!("🌐 WebSocket Service listening on ws://{}", addr);
    info!("📡 WebSocket endpoint: ws://{}/ws", addr);

    // Run server with graceful shutdown
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Create the router with WebSocket endpoint
fn create_router(service_islands: Arc<ServiceIslands>) -> Router {
    Router::new()
        .route("/ws", get(websocket_handler))
        .route("/health", get(health_handler))
        .with_state(service_islands)
}

/// WebSocket upgrade handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(service_islands): State<Arc<ServiceIslands>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, service_islands))
}

/// Handle individual WebSocket connection
async fn handle_websocket(mut socket: WebSocket, service_islands: Arc<ServiceIslands>) {
    use std::sync::atomic::Ordering;
    use futures::StreamExt;

    // Increment connection counter
    service_islands.active_ws_connections.fetch_add(1, Ordering::SeqCst);
    let current_connections = service_islands.active_connections();
    info!("➕ New WebSocket connection (total: {})", current_connections);

    // Subscribe to broadcast channel
    let mut rx = service_islands.websocket_service.broadcast_service.subscribe();

    // Send initial message
    if socket.send(Message::Text("Connected to WebSocket service".to_string())).await.is_err() {
        info!("Failed to send initial message");
        return;
    }

    // Handle incoming messages and broadcasts
    loop {
        tokio::select! {
            // Receive broadcast messages
            msg = rx.recv() => {
                match msg {
                    Ok(text) => {
                        if socket.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            // Receive client messages (we just ignore them for now)
            Some(msg) = socket.recv() => {
                if msg.is_err() {
                    break;
                }
            }
        }
    }

    // Decrement connection counter
    service_islands.active_ws_connections.fetch_sub(1, Ordering::SeqCst);
    let current_connections = service_islands.active_connections();
    info!("➖ WebSocket connection closed (total: {})", current_connections);
}

/// Health check endpoint
async fn health_handler(
    State(service_islands): State<Arc<ServiceIslands>>,
) -> impl IntoResponse {
    let healthy = service_islands.health_check().await;
    let status = if healthy { "healthy" } else { "unhealthy" };
    let status_code = if healthy {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        axum::Json(serde_json::json!({
            "status": status,
            "service": "web-server-report-websocket",
            "active_connections": service_islands.active_connections(),
        }))
    )
}

/// Background task to fetch market data periodically
async fn spawn_market_data_fetcher(service_islands: Arc<ServiceIslands>) {
    info!("🔄 Starting periodic market data fetcher...");

    // Read interval from environment variable (default: 10 seconds)
    let fetch_interval = env::var("FETCH_INTERVAL_SECONDS")
        .unwrap_or_else(|_| "10".to_string())
        .parse::<u64>()
        .unwrap_or(10);

    info!("⏱️ Market data fetch interval: {} seconds", fetch_interval);

    let mut interval_timer = interval(Duration::from_secs(fetch_interval));

    loop {
        interval_timer.tick().await;

        info!("📊 Fetching market data...");

        match service_islands.fetch_and_publish_market_data(false).await {
            Ok(data) => {
                info!("✅ Market data fetched successfully");

                // Broadcast to all WebSocket clients
                if let Err(e) = service_islands.broadcast_to_websocket_clients(data).await {
                    error!("❌ Failed to broadcast to WebSocket clients: {}", e);
                } else {
                    info!("📡 Broadcasted to {} WebSocket clients",
                          service_islands.active_connections());
                }
            }
            Err(e) => {
                error!("❌ Failed to fetch market data: {}", e);
            }
        }
    }
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("🛑 Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            info!("🛑 Received SIGTERM, shutting down gracefully...");
        },
    }
}
