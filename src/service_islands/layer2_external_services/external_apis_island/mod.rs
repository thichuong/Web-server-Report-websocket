//! External APIs Island - Layer 2: External Services (Optimized for Maximum Performance)
//!
//! This island manages all external API interactions including:
//! - Market data fetching from cryptocurrency APIs
//! - Circuit breaker protection for service resilience
//! - Data aggregation and normalization
//! - Error handling for external service calls
//!
//! PERFORMANCE OPTIMIZATION: Rate limiting completely removed for maximum throughput.
//! Cache logic handled by Layer 1, this layer focuses on pure API business logic.

pub mod market_data_api;
pub mod api_aggregator;
pub mod circuit_breaker;

use anyhow::Result;
use std::sync::Arc;

use market_data_api::MarketDataApi;
use api_aggregator::ApiAggregator;

/// External APIs Island - Main entry point for Layer 2
///
/// Combines MarketDataApi and ApiAggregator for comprehensive external API management.
pub struct ExternalApisIsland {
    pub market_api: Arc<MarketDataApi>,
    pub aggregator: Arc<ApiAggregator>,
}

impl ExternalApisIsland {
    /// Create a new ExternalApisIsland with cache system
    pub async fn with_cache_and_all_keys(
        taapi_secret: String,
        cmc_api_key: Option<String>,
        finnhub_api_key: Option<String>,
        cache_system: Option<Arc<crate::service_islands::layer1_infrastructure::CacheSystemIsland>>,
    ) -> Result<Self> {
        println!("🌐 Initializing External APIs Island...");

        // Initialize Market Data API
        let market_api = Arc::new(MarketDataApi::with_all_keys(taapi_secret.clone(), cmc_api_key.clone(), finnhub_api_key.clone()).await?);

        // Initialize API Aggregator
        let aggregator = if let Some(cache) = cache_system {
            Arc::new(ApiAggregator::with_cache_and_all_keys(
                taapi_secret,
                cmc_api_key,
                finnhub_api_key,
                cache
            ).await?)
        } else {
            Arc::new(ApiAggregator::with_all_keys(
                taapi_secret,
                cmc_api_key,
                finnhub_api_key
            ).await?)
        };

        println!("✅ External APIs Island initialized successfully");

        Ok(Self {
            market_api,
            aggregator,
        })
    }

    /// Health check for External APIs Island
    pub async fn health_check(&self) -> Result<bool> {
        let market_api_healthy = self.market_api.health_check().await;
        let aggregator_healthy = self.aggregator.health_check().await;
        Ok(market_api_healthy && aggregator_healthy)
    }

    /// Fetch dashboard summary v2 - Main Layer 2 functionality
    /// 
    /// force_realtime_refresh: If true, forces refresh of RealTime cached data
    pub async fn fetch_dashboard_summary_v2(&self, force_realtime_refresh: bool) -> Result<serde_json::Value> {
        self.aggregator.fetch_dashboard_summary_v2(force_realtime_refresh).await
    }
}