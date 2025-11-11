//! Service Islands Architecture Registry for WebSocket Service
//!
//! This module provides the service islands initialization for the WebSocket service.
//! Only includes the necessary islands for WebSocket functionality and external API access.

pub mod layer1_infrastructure;
pub mod layer2_external_services;
pub mod layer3_communication;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};

use layer1_infrastructure::{CacheSystemIsland, LeaderElectionService};
use layer2_external_services::ExternalApisIsland;
use layer3_communication::WebSocketServiceIsland;

/// WebSocket Service Islands Registry
///
/// This struct holds references to the service islands needed for the WebSocket service:
/// - Layer 1: Cache system for Redis access
/// - Layer 2: External APIs for market data fetching
/// - Layer 3: WebSocket service
pub struct ServiceIslands {
    // Layer 1: Infrastructure Islands
    pub cache_system: Arc<CacheSystemIsland>,

    // Layer 2: External Services Islands
    pub external_apis: Arc<ExternalApisIsland>,

    // Layer 3: Communication Islands
    pub websocket_service: Arc<WebSocketServiceIsland>,

    // Distributed Coordination: Leader Election
    pub leader_election: Arc<LeaderElectionService>,
    pub is_leader: Arc<AtomicBool>,

    // WebSocket connection tracking
    pub active_ws_connections: Arc<AtomicUsize>,
}

impl ServiceIslands {
    /// Initialize Service Islands for WebSocket service
    ///
    /// This method initializes only the necessary service islands:
    /// Layer 1 (Infrastructure/Cache), Layer 2 (External APIs), Layer 3 (Communication)
    pub async fn initialize() -> Result<Self, anyhow::Error> {
        println!("🏝️ Initializing WebSocket Service Islands...");

        // Initialize Layer 1: Infrastructure (Cache System only)
        println!("🏗️ Initializing Layer 1: Cache System Island...");
        let cache_system = Arc::new(CacheSystemIsland::new().await?);
        println!("✅ Cache System Island initialized!");

        // Initialize Leader Election Service
        println!("🎖️ Initializing Leader Election Service...");
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        // Generate unique node ID from Railway or UUID
        let node_id = std::env::var("RAILWAY_REPLICA_ID")
            .or_else(|_| std::env::var("RAILWAY_INSTANCE_ID"))
            .unwrap_or_else(|_| format!("ws-{}", uuid::Uuid::new_v4()));

        let leader_election = Arc::new(
            LeaderElectionService::new(&redis_url, node_id).await?
        );
        let is_leader = Arc::new(AtomicBool::new(false));

        // Spawn background leadership monitoring task
        tokio::spawn({
            let leader_election = leader_election.clone();
            let is_leader = is_leader.clone();
            async move {
                leader_election.monitor_leadership(is_leader).await;
            }
        });

        println!("✅ Leader Election Service initialized!");

        // Initialize Layer 2: External Services (depends on Layer 1 - Cache System)
        println!("🌐 Initializing Layer 2: External APIs Island with Cache...");
        let taapi_secret = std::env::var("TAAPI_SECRET").unwrap_or_else(|_| "default_secret".to_string());
        let cmc_api_key = std::env::var("CMC_API_KEY").ok();
        let finnhub_api_key = std::env::var("FINNHUB_API_KEY").ok();

        if cmc_api_key.is_some() {
            println!("🔑 CoinMarketCap API key found - enabling fallback support");
        } else {
            println!("⚠️ No CoinMarketCap API key - using CoinGecko only");
        }

        if finnhub_api_key.is_some() {
            println!("📈 Finnhub API key found - enabling US stock indices");
        } else {
            println!("⚠️ No Finnhub API key - US stock indices will be unavailable");
        }

        let external_apis = Arc::new(ExternalApisIsland::with_cache_and_all_keys(
            taapi_secret,
            cmc_api_key,
            finnhub_api_key,
            Some(cache_system.clone())
        ).await?);
        println!("✅ External APIs Island initialized!");

        // Initialize Layer 3: Communication (WebSocket)
        println!("📡 Initializing Layer 3: Communication Islands...");

        // Initialize WebSocket Service with External APIs and Cache
        let websocket_service = Arc::new(
            WebSocketServiceIsland::with_external_apis_and_cache(
                external_apis.clone(),
                cache_system.clone()
            ).await?
        );
        println!("✅ WebSocket Service Island initialized!");

        println!("✅ WebSocket Service Islands Architecture initialized!");
        println!("📊 Architecture Status:");
        println!("  🏗️ Layer 1 - Infrastructure: Cache System, Leader Election");
        println!("  🌐 Layer 2 - External Services: External APIs");
        println!("  📡 Layer 3 - Communication: WebSocket");

        Ok(Self {
            cache_system,
            external_apis,
            websocket_service,
            leader_election,
            is_leader,
            active_ws_connections: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Fetch market data from External APIs and publish to Redis Streams
    pub async fn fetch_and_publish_market_data(&self, force_refresh: bool) -> Result<serde_json::Value, anyhow::Error> {
        // Fetch data directly from External APIs
        let data = self.external_apis
            .fetch_dashboard_summary_v2(force_refresh)
            .await?;

        // Store in cache for main service to read
        if let Err(e) = self.cache_system.cache_manager()
            .set_with_strategy("latest_market_data", data.clone(),
                layer1_infrastructure::cache_system_island::cache_manager::realtime_strategy())
            .await
        {
            eprintln!("⚠️ Failed to cache market data: {}", e);
        }

        // Publish to Redis Stream for main service
        if let Err(e) = self.publish_to_redis_stream(&data).await {
            eprintln!("⚠️ Failed to publish to Redis Stream: {}", e);
        }

        Ok(data)
    }

    /// Publish data to Redis Stream
    async fn publish_to_redis_stream(&self, data: &serde_json::Value) -> Result<(), anyhow::Error> {
        // Convert JSON to string for storage in stream
        let data_str = serde_json::to_string(data)?;

        // Create stream fields
        let fields = vec![("data".to_string(), data_str)];

        // Publish to market_data_stream using cache manager's stream functionality
        // Limit stream to 1000 entries (MAXLEN)
        self.cache_system
            .cache_manager()
            .publish_to_stream("market_data_stream", fields, Some(1000))
            .await?;

        Ok(())
    }

    /// Broadcast data to all connected WebSocket clients
    pub async fn broadcast_to_websocket_clients(&self, data: serde_json::Value) -> Result<(), anyhow::Error> {
        // Wrap data in WebSocket message format with type field
        let ws_message = serde_json::json!({
            "type": "dashboard_update",
            "data": data,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "source": "external_apis"
        });

        let data_str = serde_json::to_string(&ws_message)?;
        self.websocket_service.broadcast_service.broadcast(data_str).await;
        Ok(())
    }

    /// Perform health check on all Service Islands
    pub async fn health_check(&self) -> bool {
        println!("🔍 Performing WebSocket Service Islands health check...");

        let cache_system_healthy = self.cache_system.health_check().await;
        let external_apis_healthy = self.external_apis.health_check().await.unwrap_or(false);
        let websocket_service_healthy = self.websocket_service.health_check().await.is_ok();

        let all_healthy = cache_system_healthy && external_apis_healthy && websocket_service_healthy;

        if all_healthy {
            println!("✅ All WebSocket Service Islands are healthy!");
        } else {
            println!("❌ Some WebSocket Service Islands are unhealthy!");
            println!("   Cache System Island: {}", if cache_system_healthy { "✅" } else { "❌" });
            println!("   External APIs Island: {}", if external_apis_healthy { "✅" } else { "❌" });
            println!("   WebSocket Service Island: {}", if websocket_service_healthy { "✅" } else { "❌" });
        }

        all_healthy
    }

    /// Get number of active WebSocket connections
    pub fn active_connections(&self) -> usize {
        use std::sync::atomic::Ordering;
        self.active_ws_connections.load(Ordering::SeqCst)
    }
}
