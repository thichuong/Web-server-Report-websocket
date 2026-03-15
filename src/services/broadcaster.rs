//! WebSocket Service Island - Layer 3 Communication
//!
//! This island handles all WebSocket-related functionality including:
//! - Connection management
//! - Message handling and broadcasting  
//! - Real-time data updates from Layer 2 External APIs
//! - Client communication protocols

pub mod broadcast_service;
pub mod connection_manager;
pub mod handlers;
pub mod market_data_streamer;
pub mod message_handler;

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::services::market_data::ExternalApisIsland;
use broadcast_service::BroadcastService;
use connection_manager::ConnectionManager;
use handlers::WebSocketHandlers;
use market_data_streamer::MarketDataStreamer;
use message_handler::MessageHandler;
// use crate::service_islands::layer3_communication::layer2_adapters::Layer2AdaptersHub;  // Removed - using external_apis directly

/// WebSocket Service Island
///
/// Central coordinator for all WebSocket communication functionality.
/// Manages real-time connections, message broadcasting, and data synchronization.
/// Integrates with Layer 2 External APIs following Service Islands Architecture.
pub struct WebSocketServiceIsland {
    /// Connection management component
    pub connection_manager: Arc<ConnectionManager>,
    /// Message processing component
    pub message_handler: Arc<MessageHandler>,
    /// Broadcast service component
    pub broadcast_service: Arc<BroadcastService>,
    /// HTTP handlers component
    pub handlers: Arc<WebSocketHandlers>,
    /// Market data streaming component
    pub market_data_streamer: Arc<MarketDataStreamer>,
}

impl WebSocketServiceIsland {
    /// Initialize the WebSocket Service Island with External APIs and Cache Optimization
    ///
    /// Creates all components and establishes communication channels with Layer 2 and cache optimization.
    ///
    /// # Errors
    /// Returns error if component initialization fails
    #[allow(clippy::unused_async)]
    pub async fn with_external_apis_and_cache(
        _external_apis: Arc<ExternalApisIsland>,
        _cache_system: Arc<crate::infrastructure::cache::CacheSystem>,
    ) -> Result<Self> {
        info!("Initializing WebSocket Service Island with External APIs and Cache");

        // Initialize components
        let connection_manager = Arc::new(ConnectionManager::new());
        let message_handler = Arc::new(MessageHandler::new());
        let broadcast_service = Arc::new(BroadcastService::new());
        let handlers = Arc::new(WebSocketHandlers::new());

        // Initialize market data streamer WITHOUT external APIs dependency
        // It should use layer2_adapters instead for proper architecture
        let market_data_streamer = Arc::new(MarketDataStreamer::new());

        // Start unified market data streaming via Layer 2 Adapters
        // TODO: Update MarketDataStreamer to use layer2_adapters instead of external_apis

        Ok(Self {
            connection_manager,
            message_handler,
            broadcast_service,
            handlers,
            market_data_streamer,
        })
    }

    /// Initialize the WebSocket Service Island with Layer 2 gRPC Client and Cache Optimization
    ///
    /// Note: gRPC client not used in websocket service - uses direct external API access instead
    ///
    /// # Errors
    /// Returns error if component initialization fails
    #[allow(dead_code)]
    #[allow(clippy::unused_async)]
    pub async fn with_grpc_client_and_cache(
        _layer2_grpc_client: Arc<String>, // Placeholder - not used
        _cache_system: Arc<crate::infrastructure::cache::CacheSystem>,
    ) -> Result<Self> {
        info!("Initializing WebSocket Service Island (websocket service doesn't use gRPC)");

        // Initialize components
        let connection_manager = Arc::new(ConnectionManager::new());
        let message_handler = Arc::new(MessageHandler::new());
        let broadcast_service = Arc::new(BroadcastService::new());
        let handlers = Arc::new(WebSocketHandlers::new());

        // Initialize market data streamer
        let market_data_streamer = Arc::new(MarketDataStreamer::new());

        info!("WebSocket Service Island initialized with gRPC Client");

        Ok(Self {
            connection_manager,
            message_handler,
            broadcast_service,
            handlers,
            market_data_streamer,
        })
    }

    /// Health check for the entire WebSocket Service Island
    ///
    /// Validates that all components are operational.
    ///
    /// # Errors
    /// Returns error if any component is unhealthy
    pub async fn health_check(&self) -> Result<()> {
        debug!("Checking WebSocket Service Island health");

        // Check all components
        let checks = vec![
            (
                "Connection Manager",
                self.connection_manager.health_check().await,
            ),
            ("Message Handler", self.message_handler.health_check().await),
            (
                "Broadcast Service",
                self.broadcast_service.health_check().await,
            ),
            ("WebSocket Handlers", self.handlers.health_check().await),
            (
                "Market Data Streamer",
                self.market_data_streamer.health_check().await,
            ),
        ];

        let mut all_healthy = true;
        for (component, healthy) in checks {
            if healthy {
                debug!("{} - Healthy", component);
            } else {
                warn!("{} - Unhealthy", component);
                all_healthy = false;
            }
        }

        if all_healthy {
            info!("WebSocket Service Island - All components healthy");
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "WebSocket Service Island - Some components unhealthy"
            ))
        }
    }

    /// Fetch market data (DEPRECATED - now handled by top-level `ServiceIslands`)
    ///
    /// This method is no longer used. Market data fetching is now done by
    /// `ServiceIslands.fetch_and_publish_market_data()` which uses `external_apis` directly.
    ///
    /// # Errors
    /// Always returns error as this method is deprecated
    #[allow(dead_code)]
    #[allow(clippy::unused_async)]
    pub async fn fetch_market_data(
        &self,
        _force_realtime_refresh: bool,
    ) -> Result<serde_json::Value> {
        Err(anyhow::anyhow!(
            "fetch_market_data is deprecated - use ServiceIslands.fetch_and_publish_market_data() instead"
        ))
    }
}
