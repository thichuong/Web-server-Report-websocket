//! Crypto Price Fetchers Component
//!
//! This module contains all the cryptocurrency price fetching methods with caching.

use super::aggregator_core::ApiAggregator;
use crate::dto::websocket::CryptoPrice;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

impl ApiAggregator {
    /// Fetch all crypto prices with type-safe automatic caching
    ///
    /// ✨ NEW: Uses `get_or_compute_typed()` for automatic caching
    ///
    /// Returns `HashMap` with coin symbols as keys: BTC, ETH, SOL, XRP, ADA, LINK, BNB
    /// Each value is a strongly-typed `CryptoPrice` with `price_usd` and `change_24h`
    ///
    /// `force_refresh`: If true, bypasses cache and forces API fetch, then updates cache
    ///
    /// # Errors
    /// Returns error if API fetch fails or cache operations encounter issues
    pub async fn fetch_all_crypto_prices_with_cache(
        &self,
        force_refresh: bool,
    ) -> Result<HashMap<String, CryptoPrice>> {
        let cache_key = "multi_crypto_prices_realtime";

        // Handle force refresh: bypass cache and update
        if force_refresh && let Some(ref cache) = self.cache_system {
            info!("Force refresh - fetching fresh crypto prices from API");

            // Fetch from API
            let raw_data = self.market_api.fetch_multi_crypto_prices().await?;

            // Convert HashMap<String, (f64, f64)> to HashMap<String, CryptoPrice>
            let result: HashMap<String, CryptoPrice> = raw_data
                .into_iter()
                .map(|(coin, (price_usd, change_24h))| {
                    (coin, CryptoPrice::new(price_usd, change_24h))
                })
                .collect();

            // Update cache with strongly-typed data
            if let Ok(cache_vec) = serde_json::to_vec(&result) {
                let cache_value = multi_tier_cache::Bytes::from(cache_vec);
                let _ = cache
                    .cache_manager
                    .set_with_strategy(
                        cache_key,
                        cache_value,
                        crate::infrastructure::cache::realtime_strategy(),
                    )
                    .await;
                debug!("All crypto prices cached after force refresh (RealTime - 30s TTL)");
            }

            return Ok(result);
        }

        // Normal flow: Use type-safe caching
        if let Some(ref cache) = self.cache_system {
            let market_api = Arc::clone(&self.market_api);

            let result = cache
                .cache_manager
                .get_or_compute_typed(
                    cache_key,
                    crate::infrastructure::cache::realtime_strategy(),
                    || async move {
                        debug!("Fetching all crypto prices from API");
                        let raw_data =
                            market_api.fetch_multi_crypto_prices().await.map_err(|e| {
                                multi_tier_cache::CacheError::BackendError(e.to_string())
                            })?;

                        // Convert HashMap<String, (f64, f64)> to HashMap<String, CryptoPrice>
                        let res: HashMap<String, CryptoPrice> = raw_data
                            .into_iter()
                            .map(|(coin, (price_usd, change_24h))| {
                                (coin, CryptoPrice::new(price_usd, change_24h))
                            })
                            .collect();

                        debug!("All crypto prices fetched and ready for caching");
                        Ok(res)
                    },
                )
                .await;

            result.map_err(anyhow::Error::from)
        } else {
            // No cache system - direct API call
            warn!("No cache system - calling API directly");
            let raw_data = self.market_api.fetch_multi_crypto_prices().await?;

            // Convert HashMap<String, (f64, f64)> to HashMap<String, CryptoPrice>
            let result: HashMap<String, CryptoPrice> = raw_data
                .into_iter()
                .map(|(coin, (price_usd, change_24h))| {
                    (coin, CryptoPrice::new(price_usd, change_24h))
                })
                .collect();

            Ok(result)
        }
    }
}
