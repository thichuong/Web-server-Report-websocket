//! Dashboard Aggregator Component
//!
//! This module contains the dashboard data aggregation logic that orchestrates
//! multiple API calls concurrently and handles error processing.

use anyhow::Result;
use std::sync::atomic::Ordering;
use tokio::time::{timeout, Duration};
use super::aggregator_core::ApiAggregator;

impl ApiAggregator {
    /// Fetch dashboard summary v2 - Main method for Layer 2 dashboard data
    /// Returns a focused summary with essential market data
    /// 
    /// force_realtime_refresh: If true, forces refresh of RealTime cached data (crypto prices)
    pub async fn fetch_dashboard_summary_v2(&self, force_realtime_refresh: bool) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        self.total_aggregations.fetch_add(1, Ordering::Relaxed);

        println!("🔄 Starting dashboard summary v2 aggregation...");

        // Fetch essential data concurrently with shorter timeouts for summary
        // OPTIMIZED: Single multi-crypto API call instead of 7 individual calls
        let multi_crypto_future = timeout(Duration::from_secs(8), self.fetch_all_crypto_prices_with_cache(force_realtime_refresh));
        let global_future = timeout(Duration::from_secs(8), self.fetch_global_with_cache());
        let fng_future = timeout(Duration::from_secs(8), self.fetch_fng_with_cache());
        let btc_rsi_14_future = timeout(Duration::from_secs(8), self.fetch_btc_rsi_14_with_cache());
        let us_indices_future = timeout(Duration::from_secs(8), self.fetch_us_indices_with_cache());

        let (multi_crypto_result, global_result, fng_result, btc_rsi_14_result, us_indices_result) = tokio::join!(
            multi_crypto_future,
            global_future, fng_future, btc_rsi_14_future, us_indices_future
        );

        let mut partial_failure = false;

        // Process multi-crypto data (all 7 coins in one result)
        let mut crypto_prices = std::collections::HashMap::new();
        match multi_crypto_result {
            Ok(Ok(prices_map)) => {
                crypto_prices = prices_map;
            }
            _ => {
                partial_failure = true;
                println!("⚠️ Multi-crypto prices fetch failed");
            }
        }

        // Helper structure for price data
        #[derive(Debug, Clone, Copy)]
        struct CryptoPrice {
            price_usd: f64,
            change_24h: f64,
        }

        impl CryptoPrice {
            fn from_json(data: &serde_json::Value) -> Self {
                Self {
                    price_usd: data["price_usd"].as_f64().unwrap_or(0.0),
                    change_24h: data["change_24h"].as_f64().unwrap_or(0.0),
                }
            }

            fn default() -> Self {
                Self {
                    price_usd: 0.0,
                    change_24h: 0.0,
                }
            }
        }

        // Extract price data once for each symbol
        let get_price = |symbol: &str| -> CryptoPrice {
            crypto_prices.get(symbol)
                .map(CryptoPrice::from_json)
                .unwrap_or_else(CryptoPrice::default)
        };

        // Extract individual coin data
        let btc = get_price("BTC");
        let (btc_price, btc_change) = (btc.price_usd, btc.change_24h);
        let eth = get_price("ETH");
        let (eth_price, eth_change) = (eth.price_usd, eth.change_24h);
        let sol = get_price("SOL");
        let (sol_price, sol_change) = (sol.price_usd, sol.change_24h);
        let xrp = get_price("XRP");
        let (xrp_price, xrp_change) = (xrp.price_usd, xrp.change_24h);
        let ada = get_price("ADA");
        let (ada_price, ada_change) = (ada.price_usd, ada.change_24h);
        let link = get_price("LINK");
        let (link_price, link_change) = (link.price_usd, link.change_24h);
        let bnb = get_price("BNB");
        let (bnb_price, bnb_change) = (bnb.price_usd, bnb.change_24h);

        // Process global data
        let (market_cap, volume_24h, market_cap_change, btc_dominance, eth_dominance) = match global_result {
            Ok(Ok(global_data)) => (
                global_data["market_cap"].as_f64().unwrap_or(0.0),
                global_data["volume_24h"].as_f64().unwrap_or(0.0),
                global_data["market_cap_change_percentage_24h_usd"].as_f64().unwrap_or(0.0),
                global_data["btc_market_cap_percentage"].as_f64().unwrap_or(0.0),
                global_data["eth_market_cap_percentage"].as_f64().unwrap_or(0.0)
            ),
            _ => {
                partial_failure = true;
                (0.0, 0.0, 0.0, 0.0, 0.0)
            }
        };

        // Process FNG data
        let fng_value = match fng_result {
            Ok(Ok(fng_data)) => fng_data["value"].as_u64().unwrap_or(50) as u32,
            _ => {
                partial_failure = true;
                50
            }
        };

        // Process RSI data
        let btc_rsi_14_value = match btc_rsi_14_result {
            Ok(Ok(btc_rsi_14_data)) => btc_rsi_14_data["value"].as_f64().unwrap_or(50.0),
            _ => {
                partial_failure = true;
                50.0
            }
        };

        // Process US Stock Indices data
        let us_indices = match us_indices_result {
            Ok(Ok(indices_data)) => indices_data["indices"].clone(),
            _ => {
                partial_failure = true;
                serde_json::json!({})
            }
        };

        let duration = start_time.elapsed();

        // Update statistics
        if partial_failure {
            self.partial_failures.fetch_add(1, Ordering::Relaxed);
            println!("⚠️ Dashboard summary v2 aggregated with partial failures in {}ms", duration.as_millis());
        } else {
            self.successful_aggregations.fetch_add(1, Ordering::Relaxed);
            println!("✅ Dashboard summary v2 aggregated successfully in {}ms", duration.as_millis());
        }

        // Return focused summary JSON
        Ok(serde_json::json!({
            "btc_price_usd": btc_price,
            "btc_change_24h": btc_change,
            "eth_price_usd": eth_price,
            "eth_change_24h": eth_change,
            "sol_price_usd": sol_price,
            "sol_change_24h": sol_change,
            "xrp_price_usd": xrp_price,
            "xrp_change_24h": xrp_change,
            "ada_price_usd": ada_price,
            "ada_change_24h": ada_change,
            "link_price_usd": link_price,
            "link_change_24h": link_change,
            "bnb_price_usd": bnb_price,
            "bnb_change_24h": bnb_change,
            "market_cap_usd": market_cap,
            "volume_24h_usd": volume_24h,
            "market_cap_change_percentage_24h_usd": market_cap_change,
            "btc_market_cap_percentage": btc_dominance,
            "eth_market_cap_percentage": eth_dominance,
            "fng_value": fng_value,
            "btc_rsi_14": btc_rsi_14_value,
            "us_stock_indices": us_indices,
            "fetch_duration_ms": duration.as_millis() as u64,
            "partial_failure": partial_failure,
            "last_updated": chrono::Utc::now().to_rfc3339(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}