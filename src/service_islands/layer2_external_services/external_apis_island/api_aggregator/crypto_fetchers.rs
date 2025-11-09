//! Crypto Price Fetchers Component
//!
//! This module contains all the cryptocurrency price fetching methods with caching.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use super::aggregator_core::ApiAggregator;

impl ApiAggregator {
    /// Fetch all crypto prices with type-safe automatic caching
    ///
    /// ✨ NEW: Uses get_or_compute_typed() for automatic caching
    ///
    /// Returns HashMap with coin symbols as keys: BTC, ETH, SOL, XRP, ADA, LINK, BNB
    /// Each value is a JSON object with price_usd and change_24h
    ///
    /// force_refresh: If true, bypasses cache and forces API fetch, then updates cache
    pub async fn fetch_all_crypto_prices_with_cache(&self, force_refresh: bool) -> Result<HashMap<String, serde_json::Value>> {
        let cache_key = "multi_crypto_prices_realtime";

        // Handle force refresh: bypass cache and update
        if force_refresh {
            if let Some(ref cache) = self.cache_system {
                println!("🔄 Force refresh - fetching fresh crypto prices from API");

                // Fetch from API
                let raw_data = self.market_api.fetch_multi_crypto_prices().await?;

                // Convert HashMap<String, (f64, f64)> to HashMap<String, serde_json::Value>
                let mut result = HashMap::new();
                for (coin, (price_usd, change_24h)) in raw_data {
                    result.insert(coin.clone(), serde_json::json!({
                        "price_usd": price_usd,
                        "change_24h": change_24h,
                        "source": "binance",
                        "last_updated": chrono::Utc::now().to_rfc3339()
                    }));
                }

                // Update cache
                let cache_value = serde_json::to_value(&result).unwrap_or(serde_json::json!({}));
                let _ = cache.cache_manager.set_with_strategy(cache_key, cache_value,
                    crate::service_islands::layer1_infrastructure::cache_system_island::cache_manager::CacheStrategy::RealTime).await;
                println!("💾 All crypto prices cached after force refresh (RealTime - 30s TTL)");

                return Ok(result);
            }
        }

        // Normal flow: Use type-safe caching
        if let Some(ref cache) = self.cache_system {
            let market_api = Arc::clone(&self.market_api);

            match cache.cache_manager.get_or_compute_typed(
                cache_key,
                crate::service_islands::layer1_infrastructure::cache_system_island::cache_manager::CacheStrategy::RealTime,
                || async move {
                    println!("🔄 Fetching all crypto prices from API...");
                    let raw_data = market_api.fetch_multi_crypto_prices().await?;

                    // Convert HashMap<String, (f64, f64)> to HashMap<String, serde_json::Value>
                    let mut result = HashMap::new();
                    for (coin, (price_usd, change_24h)) in raw_data {
                        result.insert(coin.clone(), serde_json::json!({
                            "price_usd": price_usd,
                            "change_24h": change_24h,
                            "source": "binance",
                            "last_updated": chrono::Utc::now().to_rfc3339()
                        }));
                    }

                    println!("✅ All crypto prices fetched and ready for caching");
                    Ok(result)
                }
            ).await {
                Ok(prices) => {
                    println!("💾 All crypto prices ready (with stampede protection)");
                    Ok(prices)
                }
                Err(e) => Err(e)
            }
        } else {
            // No cache system - direct API call
            println!("⚠️ No cache system - calling API directly");
            let raw_data = self.market_api.fetch_multi_crypto_prices().await?;

            let mut result = HashMap::new();
            for (coin, (price_usd, change_24h)) in raw_data {
                result.insert(coin.clone(), serde_json::json!({
                    "price_usd": price_usd,
                    "change_24h": change_24h,
                    "source": "binance",
                    "last_updated": chrono::Utc::now().to_rfc3339()
                }));
            }
            Ok(result)
        }
    }
}
