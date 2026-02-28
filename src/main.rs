#![warn(clippy::pedantic)]
use anyhow::Context;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{signal, time::interval};
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod api;
pub mod config;
pub mod dto;
pub mod infrastructure;
pub mod performance;
pub mod services;

use api::routes::create_router;
use api::state::AppState;
use config::app_env::AppConfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "web_server_report_websocket=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting WebSocket Service...");

    let app_config = AppConfig::load();

    // Initialize Cache
    info!("🏗️ Initializing Cache System...");
    let cache_system = Arc::new(infrastructure::cache::CacheSystem::new().await?);

    // Initialize Leader Election
    info!("🎖️ Initializing Leader Election Service...");
    let node_id = std::env::var("RAILWAY_REPLICA_ID")
        .or_else(|_| std::env::var("RAILWAY_INSTANCE_ID"))
        .unwrap_or_else(|_| format!("ws-{}", uuid::Uuid::new_v4()));
    let leader_election = Arc::new(
        services::leader_election::LeaderElectionService::new(&app_config.redis_url, node_id)
            .await?,
    );
    let is_leader = Arc::new(AtomicBool::new(false));

    tokio::spawn({
        let leader_election = leader_election.clone();
        let is_leader = is_leader.clone();
        async move {
            leader_election.monitor_leadership(is_leader).await;
        }
    });

    // Initialize External APIs
    info!("🌐 Initializing External APIs...");
    let external_apis = Arc::new(
        services::market_data::ExternalApisIsland::with_cache_and_all_keys(
            app_config.taapi_secret.clone(),
            app_config.cmc_api_key.clone(),
            app_config.finnhub_api_key.clone(),
            Some(cache_system.clone()),
        )
        .await?,
    );

    // Initialize Broadcaster
    info!("📡 Initializing WebSocket Broadcaster...");
    let broadcaster = Arc::new(
        services::broadcaster::WebSocketServiceIsland::with_external_apis_and_cache(
            external_apis.clone(),
            cache_system.clone(),
        )
        .await?,
    );

    // Build AppState
    let app_state = Arc::new(AppState {
        cache: cache_system,
        external_apis,
        broadcaster,
        leader_election: leader_election.clone(),
        is_leader,
        active_ws_connections: Arc::new(AtomicUsize::new(0)),
    });

    let (is_healthy, health_details) = app_state.health_check_detailed().await;
    if is_healthy {
        info!("✅ Service is healthy!");
    } else {
        warn!("⚠️ Some services may have issues - continuing with startup...");
        warn!("Health details: {:?}", health_details);
    }

    let state_clone = app_state.clone();
    let fetch_interval = app_config.fetch_interval;
    tokio::spawn(async move {
        spawn_market_data_fetcher(state_clone, fetch_interval).await;
    });

    let app = create_router(app_state.clone());

    let addr: SocketAddr = format!("{}:{}", app_config.host, app_config.port)
        .parse()
        .context("HOST and PORT must form a valid address")?;

    info!("🌐 WebSocket Service listening on ws://{}", addr);

    let server = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal());

    server.await?;

    info!("🔓 Releasing leadership before shutdown...");
    match app_state.leader_election.release_leadership().await { Err(e) => {
        warn!("⚠️ Failed to release leadership: {}", e);
    } _ => {
        info!("✅ Leadership released successfully");
    }}

    info!("👋 WebSocket service shutdown complete");
    Ok(())
}

async fn spawn_market_data_fetcher(state: Arc<AppState>, fetch_interval: u64) {
    use std::sync::atomic::Ordering;

    info!("🔄 Starting periodic market data fetcher with leader election...");
    info!("⏱️ Market data fetch interval: {} seconds", fetch_interval);

    let mut interval_timer = interval(Duration::from_secs(fetch_interval));

    loop {
        interval_timer.tick().await;

        let is_leader = state.is_leader.load(Ordering::Relaxed);

        if is_leader {
            info!("🎖️ [LEADER] Fetching market data from APIs...");

            match state.fetch_and_publish_market_data(true).await {
                Ok(data) => {
                    info!("✅ [LEADER] Market data fetched successfully from APIs");
                    match state.broadcast_to_websocket_clients(data).await { Err(e) => {
                        error!(
                            "❌ [LEADER] Failed to broadcast to WebSocket clients: {}",
                            e
                        );
                    } _ => {
                        info!(
                            "📡 [LEADER] Broadcasted to {} WebSocket clients",
                            state.active_connections()
                        );
                    }}
                }
                Err(e) => {
                    error!("❌ [LEADER] Failed to fetch market data: {}", e);
                }
            }
        } else {
            info!("👥 [FOLLOWER] Reading market data from cache...");

            match state.cache.cache_manager().get("latest_market_data").await {
                Ok(Some(data)) => {
                    info!("✅ [FOLLOWER] Market data loaded from cache");
                    match serde_json::from_value(data) {
                        Ok(dashboard_data) => {
                            match state.broadcast_to_websocket_clients(dashboard_data).await
                            { Err(e) => {
                                error!(
                                    "❌ [FOLLOWER] Failed to broadcast to WebSocket clients: {}",
                                    e
                                );
                            } _ => {
                                info!(
                                    "📡 [FOLLOWER] Broadcasted cached data to {} WebSocket clients",
                                    state.active_connections()
                                );
                            }}
                        }
                        Err(e) => {
                            error!("❌ [FOLLOWER] Failed to deserialize cached data: {}", e);
                        }
                    }
                }
                Ok(None) => warn!(
                    "⚠️ [FOLLOWER] No cached data available yet (leader may still be fetching)"
                ),
                Err(e) => error!("❌ [FOLLOWER] Failed to read from cache: {}", e),
            }
        }
    }
}

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
        () = ctrl_c => {
            info!("🛑 Received Ctrl+C, shutting down gracefully...");
        },
        () = terminate => {
            info!("🛑 Received SIGTERM, shutting down gracefully...");
        },
    }
}
