//! Market Data Fetchers Component
//!
//! This module contains all the market data fetching methods with caching
//! for global market data, Fear & Greed Index, RSI, and US stock indices.

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, warn};
use super::aggregator_core::ApiAggregator;

impl ApiAggregator {
    /// Fetch global data with type-safe automatic caching
    ///
    /// ✨ NEW: Uses get_or_compute_typed() for automatic caching
    pub async fn fetch_global_with_cache(&self) -> Result<serde_json::Value> {
        if let Some(ref cache) = self.cache_system {
            let market_api = Arc::clone(&self.market_api);

            cache.cache_manager.get_or_compute_typed(
                "global_coingecko_1h",
                crate::service_islands::layer1_infrastructure::cache_system_island::cache_manager::CacheStrategy::MediumTerm, // 1 hour
                || async move {
                    debug!("Fetching global data from API");
                    let data = market_api.fetch_global_data().await?;
                    debug!("Global data fetched");
                    Ok(data)
                }
            ).await
        } else {
            // No cache - direct API call
            warn!("No cache system - calling API directly for global data");
            self.market_api.fetch_global_data().await
        }
    }

    /// Fetch Fear & Greed with type-safe automatic caching
    ///
    /// ✨ NEW: Uses get_or_compute_typed() for automatic caching
    pub async fn fetch_fng_with_cache(&self) -> Result<serde_json::Value> {
        if let Some(ref cache) = self.cache_system {
            let market_api = Arc::clone(&self.market_api);

            cache.cache_manager.get_or_compute_typed(
                "fng_alternative_5m",
                crate::service_islands::layer1_infrastructure::cache_system_island::cache_manager::CacheStrategy::ShortTerm, // 5 minutes
                || async move {
                    debug!("Fetching Fear & Greed Index from API");
                    let data = market_api.fetch_fear_greed_index().await?;
                    debug!("Fear & Greed Index fetched");
                    Ok(data)
                }
            ).await
        } else {
            // No cache - direct API call
            warn!("No cache system - calling API directly for FNG");
            self.market_api.fetch_fear_greed_index().await
        }
    }

    /// Fetch RSI with type-safe automatic caching
    ///
    /// ✨ NEW: Uses get_or_compute_typed() for automatic caching
    pub async fn fetch_btc_rsi_14_with_cache(&self) -> Result<serde_json::Value> {
        if let Some(ref cache) = self.cache_system {
            let market_api = Arc::clone(&self.market_api);

            cache.cache_manager.get_or_compute_typed(
                "btc_rsi_14_taapi_3h",
                crate::service_islands::layer1_infrastructure::cache_system_island::cache_manager::CacheStrategy::LongTerm, // 3 hours
                || async move {
                    debug!("Fetching BTC RSI-14 from API");
                    let data = market_api.fetch_btc_rsi_14().await?;
                    debug!("BTC RSI-14 fetched");
                    Ok(data)
                }
            ).await
        } else {
            // No cache - direct API call
            warn!("No cache system - calling API directly for RSI");
            self.market_api.fetch_btc_rsi_14().await
        }
    }

    /// Fetch US Stock Indices with type-safe automatic caching
    ///
    /// ✨ NEW: Uses get_or_compute_typed() for automatic caching
    pub async fn fetch_us_indices_with_cache(&self) -> Result<serde_json::Value> {
        if let Some(ref cache) = self.cache_system {
            let market_api = Arc::clone(&self.market_api);

            cache.cache_manager.get_or_compute_typed(
                "us_indices_finnhub_5m",
                crate::service_islands::layer1_infrastructure::cache_system_island::cache_manager::CacheStrategy::ShortTerm, // 5 minutes
                || async move {
                    debug!("Fetching US Stock Indices from API");
                    let data = market_api.fetch_us_stock_indices().await?;
                    debug!("US Stock Indices fetched");
                    Ok(data)
                }
            ).await
        } else {
            // No cache - direct API call
            warn!("No cache system - calling API directly for US indices");
            self.market_api.fetch_us_stock_indices().await
        }
    }
}