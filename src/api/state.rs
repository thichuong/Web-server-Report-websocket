use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;

use crate::infrastructure::cache::CacheSystem;
use crate::services::broadcaster::WebSocketServiceIsland;
use crate::services::leader_election::LeaderElectionService;
use crate::services::market_data::ExternalApisIsland;
use crate::dto::websocket::DashboardData;

pub struct AppState {
    pub cache: Arc<CacheSystem>,
    pub external_apis: Arc<ExternalApisIsland>,
    pub broadcaster: Arc<WebSocketServiceIsland>,
    pub leader_election: Arc<LeaderElectionService>,
    pub is_leader: Arc<AtomicBool>,
    pub active_ws_connections: Arc<AtomicUsize>,
}

impl AppState {
    pub async fn fetch_and_publish_market_data(&self, force_refresh: bool) -> anyhow::Result<DashboardData> {
        let data = self.external_apis.fetch_dashboard_summary_v2(force_refresh).await?;
        
        if let Ok(cache_value) = serde_json::to_value(&data) {
            if let Err(e) = self.cache.cache_manager().set_with_strategy(
                "latest_market_data",
                cache_value,
                crate::infrastructure::cache::realtime_strategy(),
            ).await {
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
        self.cache.cache_manager().publish_to_stream("market_data_stream", fields, Some(1000)).await?;
        Ok(())
    }

    pub async fn broadcast_to_websocket_clients(&self, data: DashboardData) -> anyhow::Result<()> {
        let payload = crate::dto::websocket::DashboardUpdatePayload::new(data, "external_apis");
        let message = crate::dto::websocket::ServerMessage::DashboardUpdate(Box::new(payload));
        let data_str = message.to_json_string()?;
        self.broadcaster.broadcast_service.broadcast(data_str).await;
        Ok(())
    }

    pub async fn health_check_detailed(&self) -> (bool, serde_json::Value) {
        let cache_healthy = self.cache.health_check().await;
        let external_apis_healthy = self.external_apis.health_check().await.unwrap_or(false);
        let websocket_healthy = self.broadcaster.health_check().await.is_ok();
        
        let core_healthy = cache_healthy && websocket_healthy;
        let status = if core_healthy && external_apis_healthy { "healthy" } else if core_healthy { "degraded" } else { "unhealthy" };
        
        let details = serde_json::json!({
            "cache_system": cache_healthy,
            "external_apis": external_apis_healthy,
            "websocket_service": websocket_healthy,
            "status": status,
        });
        
        (core_healthy, details)
    }
    
    pub fn active_connections(&self) -> usize {
        self.active_ws_connections.load(std::sync::atomic::Ordering::SeqCst)
    }
}
