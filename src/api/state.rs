use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};

use crate::dto::websocket::DashboardData;
use crate::infrastructure::cache::CacheSystem;
use crate::services::broadcaster::Broadcaster;
use crate::services::leader_election::LeaderElectionService;
use crate::services::market_data::MarketDataService;

pub struct AppState {
    pub cache: Arc<CacheSystem>,
    pub market_data: Arc<MarketDataService>,
    pub broadcaster: Arc<Broadcaster>,
    pub leader_election: Arc<LeaderElectionService>,
    pub is_leader: Arc<AtomicBool>,
    pub active_ws_connections: Arc<AtomicUsize>,
}

impl AppState {
    /// Fetches and publishes market data to cache and Redis stream.
    ///
    /// # Errors
    ///
    /// Returns an error if the market data service fails to fetch the dashboard summary,
    /// or if it fails to serialize the data to JSON string for the Redis stream.
    pub async fn fetch_and_publish_market_data(
        &self,
        force_refresh: bool,
    ) -> anyhow::Result<DashboardData> {
        let data = self.market_data.fetch_dashboard_data(force_refresh).await?;

        if let Ok(cache_vec) = serde_json::to_vec(&data) {
            let cache_value = multi_tier_cache::Bytes::from(cache_vec);
            if let Err(e) = self
                .cache
                .cache_manager()
                .set_with_strategy(
                    "latest_market_data",
                    cache_value,
                    crate::infrastructure::cache::realtime_strategy(),
                )
                .await
            {
                tracing::warn!("Failed to cache market data: {}", e);
            }
        }

        if let Err(e) = self.publish_to_redis_stream(&data).await {
            tracing::warn!("Failed to publish to Redis Stream: {}", e);
        }
        Ok(data)
    }

    async fn publish_to_redis_stream(&self, data: &DashboardData) -> anyhow::Result<()> {
        let data_str = data.to_json_string()?;
        let fields = vec![("data".to_string(), data_str)];
        self.cache
            .cache_manager()
            .publish_to_stream("market_data_stream", fields, Some(1000))
            .await?;
        Ok(())
    }

    /// Broadcasts updated dashboard data to connected WebSocket clients.
    ///
    /// # Errors
    ///
    /// Returns an error if the payload cannot be serialized to a JSON string.
    pub fn broadcast_to_websocket_clients(&self, data: DashboardData) -> anyhow::Result<()> {
        let payload = crate::dto::websocket::DashboardUpdatePayload::new(data, "market_data");
        let message = crate::dto::websocket::ServerMessage::DashboardUpdate(Box::new(payload));
        let data_str = message.to_json_string()?;
        self.broadcaster.broadcast(&data_str);
        Ok(())
    }

    pub async fn health_check_detailed(&self) -> (bool, serde_json::Value) {
        let cache_healthy = self.cache.health_check().await;
        let market_data_healthy = self.market_data.health_check();
        let websocket_healthy = self.broadcaster.health_check();

        let core_healthy = cache_healthy && websocket_healthy;
        let status = if core_healthy && market_data_healthy {
            "healthy"
        } else if core_healthy {
            "degraded"
        } else {
            "unhealthy"
        };

        let details = serde_json::json!({
            "cache_system": cache_healthy,
            "market_data": market_data_healthy,
            "websocket_service": websocket_healthy,
            "status": status,
        });

        (core_healthy, details)
    }

    #[must_use]
    pub fn active_connections(&self) -> usize {
        self.active_ws_connections
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}
