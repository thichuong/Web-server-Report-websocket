//! Dashboard Aggregator Component
//!
//! This module contains the dashboard data aggregation logic that orchestrates
//! multiple API calls concurrently and handles error processing.

use super::aggregator_core::ApiAggregator;
use crate::dto::websocket::{CryptoPrice, DashboardData, UsStockIndices};
use anyhow::Result;
use std::sync::atomic::Ordering;
use tokio::time::{Duration, timeout};
use tracing::{info, warn};

impl ApiAggregator {
    /// Fetch dashboard summary v2 - Main method for Layer 2 dashboard data
    /// Returns a focused summary with essential market data
    ///
    /// `force_realtime_refresh`: If true, forces refresh of `RealTime` cached data (crypto prices)
    ///
    /// # Errors
    /// Returns error if all API calls fail or dashboard data cannot be constructed
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::cast_possible_truncation)]
    pub async fn fetch_dashboard_summary_v2(
        &self,
        force_realtime_refresh: bool,
    ) -> Result<DashboardData> {
        let start_time = std::time::Instant::now();
        self.total_aggregations.fetch_add(1, Ordering::Relaxed);

        info!("Starting dashboard summary v2 aggregation");

        // Fetch essential data concurrently with shorter timeouts for summary
        // OPTIMIZED: Single multi-crypto API call instead of 7 individual calls
        let multi_crypto_future = timeout(
            Duration::from_secs(8),
            self.fetch_all_crypto_prices_with_cache(force_realtime_refresh),
        );
        let global_future = timeout(Duration::from_secs(8), self.fetch_global_with_cache());
        let fng_future = timeout(Duration::from_secs(8), self.fetch_fng_with_cache());
        let btc_rsi_14_future = timeout(Duration::from_secs(8), self.fetch_btc_rsi_14_with_cache());
        let us_indices_future = timeout(Duration::from_secs(8), self.fetch_us_indices_with_cache());

        let (multi_crypto_result, global_result, fng_result, btc_rsi_14_result, us_indices_result) = tokio::join!(
            multi_crypto_future,
            global_future,
            fng_future,
            btc_rsi_14_future,
            us_indices_future
        );

        let mut partial_failure = false;

        // Process multi-crypto data (all 7 coins in one result) - Now strongly-typed!
        let crypto_prices = if let Ok(Ok(prices_map)) = multi_crypto_result {
            prices_map
        } else {
            partial_failure = true;
            warn!("Multi-crypto prices fetch failed");
            std::collections::HashMap::new()
        };

        // Extract price data once for each symbol - Now directly from CryptoPrice!
        let get_price = |symbol: &str| -> CryptoPrice {
            crypto_prices.get(symbol).copied().unwrap_or_default()
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

        // Process global data - Fixed: Safe indexing with .get()
        let (market_cap, volume_24h, market_cap_change, btc_dominance, eth_dominance) =
            if let Ok(Ok(global_data)) = global_result {
                (
                    global_data
                        .get("market_cap")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0),
                    global_data
                        .get("volume_24h")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0),
                    global_data
                        .get("market_cap_change_percentage_24h_usd")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0),
                    global_data
                        .get("btc_market_cap_percentage")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0),
                    global_data
                        .get("eth_market_cap_percentage")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0),
                )
            } else {
                partial_failure = true;
                (0.0, 0.0, 0.0, 0.0, 0.0)
            };

        // Process FNG data - Fixed: Safe indexing with .get()
        let fng_value = if let Ok(Ok(fng_data)) = fng_result {
            fng_data
                .get("value")
                .and_then(serde_json::Value::as_u64)
                .map_or(50, |v| v as u32)
        } else {
            partial_failure = true;
            50
        };

        // Process RSI data - Fixed: Safe indexing with .get()
        let btc_rsi_14_value = if let Ok(Ok(btc_rsi_14_data)) = btc_rsi_14_result {
            btc_rsi_14_data
                .get("value")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(50.0)
        } else {
            partial_failure = true;
            50.0
        };

        // Process US Stock Indices data - Parse strongly-typed structure
        // Fixed: Safe indexing with .get()
        let us_stock_indices = if let Ok(Ok(indices_data)) = us_indices_result {
            // Try to parse the nested "indices" field into UsStockIndices
            if let Some(indices_value) = indices_data.get("indices") {
                match serde_json::from_value::<UsStockIndices>(indices_value.clone()) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        warn!("Failed to parse US stock indices: {}. Using empty data.", e);
                        partial_failure = true;
                        UsStockIndices::default()
                    }
                }
            } else {
                warn!("Missing 'indices' field in response. Using empty data.");
                partial_failure = true;
                UsStockIndices::default()
            }
        } else {
            partial_failure = true;
            UsStockIndices::default()
        };

        let duration = start_time.elapsed();
        let now = chrono::Utc::now().to_rfc3339();

        // Update statistics
        if partial_failure {
            self.partial_failures.fetch_add(1, Ordering::Relaxed);
            warn!(
                duration_ms = duration.as_millis(),
                "Dashboard summary v2 aggregated with partial failures"
            );
        } else {
            self.successful_aggregations.fetch_add(1, Ordering::Relaxed);
            info!(
                duration_ms = duration.as_millis(),
                "Dashboard summary v2 aggregated successfully"
            );
        }

        // Return strongly-typed DashboardData
        Ok(DashboardData {
            btc_price_usd: btc_price,
            btc_change_24h: btc_change,
            btc_market_cap_percentage: btc_dominance,
            btc_rsi_14: btc_rsi_14_value,
            eth_price_usd: eth_price,
            eth_change_24h: eth_change,
            eth_market_cap_percentage: eth_dominance,
            sol_price_usd: sol_price,
            sol_change_24h: sol_change,
            xrp_price_usd: xrp_price,
            xrp_change_24h: xrp_change,
            ada_price_usd: ada_price,
            ada_change_24h: ada_change,
            link_price_usd: link_price,
            link_change_24h: link_change,
            bnb_price_usd: bnb_price,
            bnb_change_24h: bnb_change,
            market_cap_usd: market_cap,
            volume_24h_usd: volume_24h,
            market_cap_change_percentage_24h_usd: market_cap_change,
            fng_value,
            us_stock_indices,
            fetch_duration_ms: duration.as_millis() as u64,
            partial_failure,
            last_updated: now.clone(),
            timestamp: now,
        })
    }
}
