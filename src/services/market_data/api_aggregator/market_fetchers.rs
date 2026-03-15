//! Market Data Fetchers Component
//!
//! This module contains all the market data fetching methods with caching
//! for global market data, Fear & Greed Index, RSI, and US stock indices.

use super::aggregator_core::ApiAggregator;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, warn};

impl ApiAggregator {
    /// Fetch global data with type-safe automatic caching
    ///
    /// ✨ NEW: Uses `get_or_compute_typed()` for automatic caching
    ///
    /// # Errors
    /// Returns error if API fetch fails or cache operations encounter issues
    pub async fn fetch_global_with_cache(&self) -> Result<serde_json::Value> {
        if let Some(ref cache) = self.cache_system {
            let market_api = Arc::clone(&self.market_api);

            let result = cache
                .cache_manager
                .get_or_compute_typed(
                    "global_coingecko_1h",
                    crate::infrastructure::cache::CacheStrategy::LongTerm, // 1 hour
                    || async move {
                        debug!("Fetching global data from API");
                        market_api
                            .fetch_global_data()
                            .await
                            .map_err(|e| multi_tier_cache::CacheError::BackendError(e.to_string()))
                    },
                )
                .await;
            result.map_err(anyhow::Error::from)
        } else {
            // No cache - direct API call
            warn!("No cache system - calling API directly for global data");
            self.market_api.fetch_global_data().await
        }
    }

    /// Fetch Fear & Greed with type-safe automatic caching
    ///
    /// ✨ NEW: Uses `get_or_compute_typed()` for automatic caching
    ///
    /// # Errors
    /// Returns error if API fetch fails or cache operations encounter issues
    pub async fn fetch_fng_with_cache(&self) -> Result<serde_json::Value> {
        if let Some(ref cache) = self.cache_system {
            let market_api = Arc::clone(&self.market_api);

            let result = cache
                .cache_manager
                .get_or_compute_typed(
                    "fng_alternative_5m",
                    crate::infrastructure::cache::CacheStrategy::ShortTerm, // 5 minutes
                    || async move {
                        debug!("Fetching Fear & Greed Index from API");
                        market_api
                            .fetch_fear_greed_index()
                            .await
                            .map_err(|e| multi_tier_cache::CacheError::BackendError(e.to_string()))
                    },
                )
                .await;
            result.map_err(anyhow::Error::from)
        } else {
            // No cache - direct API call
            warn!("No cache system - calling API directly for FNG");
            self.market_api.fetch_fear_greed_index().await
        }
    }

    /// Fetch RSI with type-safe automatic caching
    ///
    /// ✨ NEW: Uses `get_or_compute_typed()` for automatic caching
    ///
    /// # Errors
    /// Returns error if API fetch fails or cache operations encounter issues
    pub async fn fetch_btc_rsi_14_with_cache(&self) -> Result<serde_json::Value> {
        if let Some(ref cache) = self.cache_system {
            let market_api = Arc::clone(&self.market_api);

            let result = cache
                .cache_manager
                .get_or_compute_typed(
                    "btc_rsi_14_taapi_3h",
                    crate::infrastructure::cache::CacheStrategy::LongTerm, // 3 hours
                    || async move {
                        debug!("Fetching BTC RSI-14 from API");
                        market_api
                            .fetch_btc_rsi_14()
                            .await
                            .map_err(|e| multi_tier_cache::CacheError::BackendError(e.to_string()))
                    },
                )
                .await;
            result.map_err(anyhow::Error::from)
        } else {
            // No cache - direct API call
            warn!("No cache system - calling API directly for RSI");
            self.market_api.fetch_btc_rsi_14().await
        }
    }

    /// Fetch US Stock Indices with type-safe automatic caching
    ///
    /// ✨ NEW: Uses `get_or_compute_typed()` for automatic caching
    ///
    /// # Errors
    /// Returns error if API fetch fails or cache operations encounter issues
    pub async fn fetch_us_indices_with_cache(&self) -> Result<serde_json::Value> {
        if let Some(ref cache) = self.cache_system {
            let market_api = Arc::clone(&self.market_api);

            let result = cache
                .cache_manager
                .get_or_compute_typed(
                    "us_indices_finnhub_5m",
                    crate::infrastructure::cache::CacheStrategy::ShortTerm, // 5 minutes
                    || async move {
                        debug!("Fetching US Stock Indices from API");
                        market_api
                            .fetch_us_stock_indices()
                            .await
                            .map_err(|e| multi_tier_cache::CacheError::BackendError(e.to_string()))
                    },
                )
                .await;
            result.map_err(anyhow::Error::from)
        } else {
            // No cache - direct API call
            warn!("No cache system - calling API directly for US indices");
            self.market_api.fetch_us_stock_indices().await
        }
    }
}
