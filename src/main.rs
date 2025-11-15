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
use anyhow::Context;

mod service_islands;
mod performance;

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
    let (is_healthy, health_details) = service_islands.health_check_detailed().await;
    if is_healthy {
        info!("✅ Service Islands Architecture is healthy!");
    } else {
        warn!("⚠️ Some Service Islands may have issues - continuing with startup...");
        warn!("Health details: {:?}", health_details);
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
        .context("PORT must be a valid number")?;

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .context("HOST and PORT must form a valid address")?;

    info!("🌐 WebSocket Service listening on ws://{}", addr);
    info!("📡 WebSocket endpoint: ws://{}/ws", addr);

    // Run server with graceful shutdown
    let server = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal());

    // Wait for server to finish
    server.await?;

    // Gracefully release leadership on shutdown
    info!("🔓 Releasing leadership before shutdown...");
    if let Err(e) = service_islands.leader_election.release_leadership().await {
        warn!("⚠️ Failed to release leadership: {}", e);
    } else {
        info!("✅ Leadership released successfully");
    }

    info!("👋 WebSocket service shutdown complete");

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
/// Returns OK (200) if core services are healthy (cache, websocket)
/// External APIs being down won't fail the health check
async fn health_handler(
    State(service_islands): State<Arc<ServiceIslands>>,
) -> impl IntoResponse {
    let (is_healthy, health_details) = service_islands.health_check_detailed().await;

    let status = if is_healthy { "healthy" } else { "unhealthy" };
    let status_code = if is_healthy {
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
            "details": health_details,
        }))
    )
}

/// Background task to fetch market data periodically
///
/// With leader election enabled:
/// - Only the LEADER instance fetches from external APIs
/// - Follower instances read from Redis cache
/// - This reduces API calls and prevents rate limiting
async fn spawn_market_data_fetcher(service_islands: Arc<ServiceIslands>) {
    use std::sync::atomic::Ordering;

    info!("🔄 Starting periodic market data fetcher with leader election...");

    // Read interval from environment variable (default: 5 seconds for real-time updates)
    let fetch_interval = env::var("FETCH_INTERVAL_SECONDS")
        .unwrap_or_else(|_| "5".to_string())
        .parse::<u64>()
        .unwrap_or(5);

    info!("⏱️ Market data fetch interval: {} seconds", fetch_interval);

    let mut interval_timer = interval(Duration::from_secs(fetch_interval));

    loop {
        interval_timer.tick().await;

        // Check if this instance is the leader
        let is_leader = service_islands.is_leader.load(Ordering::Relaxed);

        if is_leader {
            // LEADER MODE: Fetch from API and cache
            info!("🎖️ [LEADER] Fetching market data from APIs...");

            match service_islands.fetch_and_publish_market_data(true).await {
                Ok(data) => {
                    info!("✅ [LEADER] Market data fetched successfully from APIs");

                    // Broadcast to all WebSocket clients
                    if let Err(e) = service_islands.broadcast_to_websocket_clients(data).await {
                        error!("❌ [LEADER] Failed to broadcast to WebSocket clients: {}", e);
                    } else {
                        info!("📡 [LEADER] Broadcasted to {} WebSocket clients",
                              service_islands.active_connections());
                    }
                }
                Err(e) => {
                    error!("❌ [LEADER] Failed to fetch market data: {}", e);
                }
            }
        } else {
            // FOLLOWER MODE: Read from cache only
            info!("👥 [FOLLOWER] Reading market data from cache...");

            // Try to get latest data from cache
            match service_islands.cache_system.cache_manager()
                .get("latest_market_data")
                .await
            {
                Ok(Some(data)) => {
                    info!("✅ [FOLLOWER] Market data loaded from cache");

                    // Broadcast to all WebSocket clients
                    if let Err(e) = service_islands.broadcast_to_websocket_clients(data).await {
                        error!("❌ [FOLLOWER] Failed to broadcast to WebSocket clients: {}", e);
                    } else {
                        info!("📡 [FOLLOWER] Broadcasted cached data to {} WebSocket clients",
                              service_islands.active_connections());
                    }
                }
                Ok(None) => {
                    warn!("⚠️ [FOLLOWER] No cached data available yet (leader may still be fetching)");
                }
                Err(e) => {
                    error!("❌ [FOLLOWER] Failed to read from cache: {}", e);
                }
            }
        }
    }
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to install Ctrl+C handler: {}", e);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                error!("Failed to install SIGTERM handler: {}", e);
            }
        }
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
