//! Market Data Service Module
//!
//! Orchestrates cryptocurrency data collection from real-time WebSocket feeds,
//! external HTTP APIs, and the multi-tier caching system to produce unified `DashboardData`.

use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, info, warn};

use super::binance_ws::BinanceWsClient;
use super::http_client::{GlobalMarketMetrics, MarketDataHttpClient};
use crate::dto::websocket::{CryptoPrice, DashboardData};
use crate::infrastructure::cache::{CacheStrategy, CacheSystem, realtime_strategy};
use crate::performance::OPTIMIZED_HTTP_CLIENT;

/// Core service for fetching and aggregating cryptocurrency market data.
pub struct MarketDataService {
    binance_ws: Arc<BinanceWsClient>,
    http_client: Arc<MarketDataHttpClient>,
    cache: Arc<CacheSystem>,
    taapi_secret: String,
    cmc_api_key: Option<String>,
    binance_rest_url: String,
}

impl MarketDataService {
    /// Creates and initializes a new `MarketDataService`.
    ///
    /// # Errors
    /// Returns error if client setup or connectivity checks fail.
    pub async fn new(
        taapi_secret: String,
        cmc_api_key: Option<String>,
        cache: Arc<CacheSystem>,
    ) -> Result<Self> {
        info!("Initializing MarketDataService...");

        if taapi_secret.is_empty() || taapi_secret == "default_secret" {
            warn!(
                "⚠️ TAAPI_SECRET is not configured or using default - BTC RSI-14 will default to neutral 50.0"
            );
        }
        if cmc_api_key.is_none() {
            warn!(
                "⚠️ CMC_API_KEY is not configured - CoinMarketCap fallback for global market data is disabled"
            );
        }

        let http_client = Arc::new(MarketDataHttpClient::new(OPTIMIZED_HTTP_CLIENT.clone()));
        let binance_rest_url = http_client.check_binance_connectivity().await.to_string();
        let binance_ws = BinanceWsClient::new();

        Ok(Self {
            binance_ws,
            http_client,
            cache,
            taapi_secret,
            cmc_api_key,
            binance_rest_url,
        })
    }

    /// Fetches all essential dashboard data concurrently with cache optimization and fallbacks.
    ///
    /// # Errors
    /// Returns error if data aggregation fails.
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::too_many_lines)]
    pub async fn fetch_dashboard_data(&self, force_refresh: bool) -> Result<DashboardData> {
        let start_time = Instant::now();
        info!("Aggregating dashboard market data (force_refresh: {force_refresh})...");

        let fetch_timeout = Duration::from_secs(8);

        let prices_fut = timeout(fetch_timeout, self.fetch_crypto_prices(force_refresh));
        let global_fut = timeout(fetch_timeout, self.fetch_global_market(force_refresh));
        let fng_fut = timeout(fetch_timeout, self.fetch_fear_greed(force_refresh));
        let rsi_fut = timeout(fetch_timeout, self.fetch_rsi(force_refresh));

        let (prices_res, global_res, fng_res, rsi_res) =
            tokio::join!(prices_fut, global_fut, fng_fut, rsi_fut);

        let mut partial_failure = false;

        // 1. Process Crypto Prices
        let crypto_prices = match prices_res {
            Ok(Ok(prices)) => prices,
            Ok(Err(e)) => {
                partial_failure = true;
                warn!("Crypto prices fetch error: {e}");
                HashMap::new()
            }
            Err(_) => {
                partial_failure = true;
                warn!("Crypto prices fetch timed out");
                HashMap::new()
            }
        };

        let get_price = |symbol: &str| -> CryptoPrice {
            crypto_prices.get(symbol).copied().unwrap_or_default()
        };

        let btc = get_price("BTC");
        let eth = get_price("ETH");
        let sol = get_price("SOL");
        let xrp = get_price("XRP");
        let ada = get_price("ADA");
        let link = get_price("LINK");
        let bnb = get_price("BNB");

        // 2. Process Global Market Data
        let global = match global_res {
            Ok(Ok(metrics)) => metrics,
            Ok(Err(e)) => {
                partial_failure = true;
                warn!("Global market metrics fetch error: {e}");
                GlobalMarketMetrics {
                    market_cap_usd: 0.0,
                    volume_24h_usd: 0.0,
                    market_cap_change_percentage_24h_usd: 0.0,
                    btc_market_cap_percentage: 0.0,
                    eth_market_cap_percentage: 0.0,
                }
            }
            Err(_) => {
                partial_failure = true;
                warn!("Global market metrics fetch timed out");
                GlobalMarketMetrics {
                    market_cap_usd: 0.0,
                    volume_24h_usd: 0.0,
                    market_cap_change_percentage_24h_usd: 0.0,
                    btc_market_cap_percentage: 0.0,
                    eth_market_cap_percentage: 0.0,
                }
            }
        };

        // 3. Process Fear & Greed
        let fng_value = match fng_res {
            Ok(Ok(val)) => val,
            Ok(Err(e)) => {
                partial_failure = true;
                warn!("Fear & Greed fetch error: {e}");
                50
            }
            Err(_) => {
                partial_failure = true;
                warn!("Fear & Greed fetch timed out");
                50
            }
        };

        // 4. Process RSI
        let btc_rsi_14 = match rsi_res {
            Ok(Ok(val)) => val,
            Ok(Err(e)) => {
                partial_failure = true;
                warn!("BTC RSI-14 fetch error: {e}");
                50.0
            }
            Err(_) => {
                partial_failure = true;
                warn!("BTC RSI-14 fetch timed out");
                50.0
            }
        };

        let duration = start_time.elapsed();
        let now = Utc::now().to_rfc3339();

        if partial_failure {
            warn!(
                duration_ms = duration.as_millis(),
                "Dashboard data aggregated with partial failures"
            );
        } else {
            info!(
                duration_ms = duration.as_millis(),
                "Dashboard data aggregated successfully"
            );
        }

        Ok(DashboardData {
            btc_price_usd: btc.price_usd,
            btc_change_24h: btc.change_24h,
            btc_market_cap_percentage: global.btc_market_cap_percentage,
            btc_rsi_14,
            eth_price_usd: eth.price_usd,
            eth_change_24h: eth.change_24h,
            eth_market_cap_percentage: global.eth_market_cap_percentage,
            sol_price_usd: sol.price_usd,
            sol_change_24h: sol.change_24h,
            xrp_price_usd: xrp.price_usd,
            xrp_change_24h: xrp.change_24h,
            ada_price_usd: ada.price_usd,
            ada_change_24h: ada.change_24h,
            link_price_usd: link.price_usd,
            link_change_24h: link.change_24h,
            bnb_price_usd: bnb.price_usd,
            bnb_change_24h: bnb.change_24h,
            market_cap_usd: global.market_cap_usd,
            volume_24h_usd: global.volume_24h_usd,
            market_cap_change_percentage_24h_usd: global.market_cap_change_percentage_24h_usd,
            fng_value,
            fetch_duration_ms: duration.as_millis() as u64,
            partial_failure,
            last_updated: now.clone(),
            timestamp: now,
        })
    }

    /// Fetches 7 crypto prices using in-memory WebSocket feed first, with HTTP REST fallback and caching,
    /// falling back to the latest Redis stream data on failure.
    async fn fetch_crypto_prices(
        &self,
        force_refresh: bool,
    ) -> Result<HashMap<String, CryptoPrice>> {
        // Fast path: In-memory real-time WebSocket cache (<30s fresh)
        if !force_refresh {
            if let Some(ws_prices) = self.binance_ws.get_multi_crypto_prices() {
                debug!("Obtained fresh real-time crypto prices from Binance WebSocket cache");
                return Ok(ws_prices);
            }
            debug!("WebSocket cache stale or incomplete, falling back to cached/REST endpoint");
        }

        let cache_key = "multi_crypto_prices_realtime";

        let fetch_res: Result<HashMap<String, CryptoPrice>> = if force_refresh {
            let res = self
                .http_client
                .fetch_multi_crypto_prices(&self.binance_rest_url)
                .await;
            match res {
                Ok(prices) => {
                    self.binance_ws.seed_prices(&prices);
                    if let Ok(vec) = serde_json::to_vec(&prices) {
                        let _ = self
                            .cache
                            .cache_manager()
                            .set_with_strategy(
                                cache_key,
                                multi_tier_cache::Bytes::from(vec),
                                realtime_strategy(),
                            )
                            .await;
                    }
                    Ok(prices)
                }
                Err(e) => Err(e),
            }
        } else {
            let http_client = Arc::clone(&self.http_client);
            let binance_ws = Arc::clone(&self.binance_ws);
            let rest_url = self.binance_rest_url.clone();

            let result = self
                .cache
                .cache_manager()
                .get_or_compute_typed(cache_key, realtime_strategy(), || async move {
                    let prices = http_client
                        .fetch_multi_crypto_prices(&rest_url)
                        .await
                        .map_err(|e| multi_tier_cache::CacheError::BackendError(e.to_string()))?;
                    binance_ws.seed_prices(&prices);
                    Ok(prices)
                })
                .await;

            result.map_err(anyhow::Error::from)
        };

        match fetch_res {
            Ok(prices) => Ok(prices),
            Err(e) => {
                warn!(
                    "Fetch crypto prices failed: {e}. Falling back to latest Redis stream data..."
                );
                match self.read_latest_dashboard_from_stream().await {
                    Ok(stream_data) => {
                        info!("Recovered crypto prices from Redis stream fallback");
                        Ok(Self::extract_prices_from_dashboard(&stream_data))
                    }
                    Err(fallback_err) => {
                        warn!("Redis stream fallback for crypto prices failed: {fallback_err}");
                        Err(e)
                    }
                }
            }
        }
    }

    /// Fetches global market data with 1-hour caching and CMC fallback, falling back to Redis stream on error.
    async fn fetch_global_market(&self, _force_refresh: bool) -> Result<GlobalMarketMetrics> {
        let http_client = Arc::clone(&self.http_client);
        let cmc_key = self.cmc_api_key.clone();

        let result = self
            .cache
            .cache_manager()
            .get_or_compute_typed(
                "global_market_metrics_1h",
                CacheStrategy::LongTerm,
                || async move {
                    match http_client.fetch_global_coingecko().await {
                        Ok(metrics) => Ok(metrics),
                        Err(e) => {
                            warn!("CoinGecko global fetch failed: {e}. Attempting CoinMarketCap fallback...");
                            if let Some(key) = &cmc_key {
                                http_client
                                    .fetch_global_cmc(key)
                                    .await
                                    .map_err(|cmc_e| multi_tier_cache::CacheError::BackendError(cmc_e.to_string()))
                            } else {
                                Err(multi_tier_cache::CacheError::BackendError(e.to_string()))
                            }
                        }
                    }
                },
            )
            .await;

        match result {
            Ok(metrics) => Ok(metrics),
            Err(e) => {
                warn!(
                    "Global market metrics fetch error: {e}. Attempting Redis stream fallback..."
                );
                match self.read_latest_dashboard_from_stream().await {
                    Ok(stream_data) => {
                        info!("Recovered global market metrics from Redis stream");
                        Ok(GlobalMarketMetrics {
                            market_cap_usd: stream_data.market_cap_usd,
                            volume_24h_usd: stream_data.volume_24h_usd,
                            market_cap_change_percentage_24h_usd: stream_data
                                .market_cap_change_percentage_24h_usd,
                            btc_market_cap_percentage: stream_data.btc_market_cap_percentage,
                            eth_market_cap_percentage: stream_data.eth_market_cap_percentage,
                        })
                    }
                    Err(stream_err) => {
                        warn!(
                            "Redis stream fallback for global market metrics failed: {stream_err}"
                        );
                        Err(anyhow::Error::from(e))
                    }
                }
            }
        }
    }

    /// Fetches Crypto Fear & Greed Index with 5-minute caching, falling back to Redis stream on error.
    async fn fetch_fear_greed(&self, _force_refresh: bool) -> Result<u32> {
        let http_client = Arc::clone(&self.http_client);

        let result = self
            .cache
            .cache_manager()
            .get_or_compute_typed(
                "fng_alternative_5m",
                CacheStrategy::ShortTerm,
                || async move {
                    http_client
                        .fetch_fear_greed()
                        .await
                        .map_err(|e| multi_tier_cache::CacheError::BackendError(e.to_string()))
                },
            )
            .await;

        match result {
            Ok(val) => Ok(val),
            Err(e) => {
                warn!("Fear & Greed fetch error: {e}. Attempting Redis stream fallback...");
                match self.read_latest_dashboard_from_stream().await {
                    Ok(stream_data) => {
                        info!(
                            "Recovered Fear & Greed from Redis stream: {}",
                            stream_data.fng_value
                        );
                        Ok(stream_data.fng_value)
                    }
                    Err(stream_err) => {
                        warn!("Redis stream fallback for Fear & Greed failed: {stream_err}");
                        Err(anyhow::Error::from(e))
                    }
                }
            }
        }
    }

    /// Fetches BTC 14-day RSI with 1-hour caching (`MediumTerm`), falling back to Redis stream on error.
    async fn fetch_rsi(&self, _force_refresh: bool) -> Result<f64> {
        let http_client = Arc::clone(&self.http_client);
        let secret = self.taapi_secret.clone();
        let is_secret_missing = secret.is_empty() || secret == "default_secret";

        let result = self
            .cache
            .cache_manager()
            .get_or_compute_typed(
                "btc_rsi_14_taapi_1h",
                CacheStrategy::MediumTerm,
                || async move {
                    if is_secret_missing {
                        return Err(multi_tier_cache::CacheError::BackendError(
                            "TAAPI secret not configured".to_string(),
                        ));
                    }
                    http_client
                        .fetch_btc_rsi_14(&secret)
                        .await
                        .map_err(|e| multi_tier_cache::CacheError::BackendError(e.to_string()))
                },
            )
            .await;

        match result {
            Ok(val) if val > 0.0 => Ok(val),
            _ => {
                // If get_or_compute_typed failed or returned invalid value, check raw Redis cache
                if let Some(cached_rsi) = self.get_cached_rsi().await {
                    debug!(
                        "Obtained BTC RSI-14 from Redis cache key 'btc_rsi_14_taapi_1h': {cached_rsi}"
                    );
                    return Ok(cached_rsi);
                }

                // Fallback to Redis stream if available
                warn!("Attempting Redis stream fallback for BTC RSI-14...");
                let rsi = match self.read_latest_dashboard_from_stream().await {
                    Ok(stream_data) if stream_data.btc_rsi_14 > 0.0 => {
                        info!(
                            "Recovered BTC RSI-14 from Redis stream: {}",
                            stream_data.btc_rsi_14
                        );
                        stream_data.btc_rsi_14
                    }
                    _ => {
                        debug!(
                            "TAAPI secret not configured and no cache/stream data available, defaulting to neutral RSI (50.0)"
                        );
                        50.0
                    }
                };

                if let Ok(vec) = serde_json::to_vec(&rsi)
                    && let Err(e) = self
                        .cache
                        .cache_manager()
                        .set_with_strategy(
                            "btc_rsi_14_taapi_1h",
                            multi_tier_cache::Bytes::from(vec),
                            CacheStrategy::MediumTerm,
                        )
                        .await
                {
                    debug!("Failed to cache fallback BTC RSI-14: {e}");
                }

                Ok(rsi)
            }
        }
    }

    /// Helper: Read cached RSI from Redis key `btc_rsi_14_taapi_1h` if present in any format.
    async fn get_cached_rsi(&self) -> Option<f64> {
        if let Ok(Some(val)) = self
            .cache
            .cache_manager()
            .get_typed::<f64>("btc_rsi_14_taapi_1h")
            .await
            && val > 0.0
        {
            return Some(val);
        }

        if let Ok(Some(bytes)) = self.cache.cache_manager().get("btc_rsi_14_taapi_1h").await
            && let Ok(s) = std::str::from_utf8(&bytes)
        {
            let trimmed = s.trim().trim_matches('"');
            if let Ok(val) = trimmed.parse::<f64>()
                && val > 0.0
            {
                return Some(val);
            }
            if let Ok(val_obj) = serde_json::from_str::<serde_json::Value>(s)
                && let Some(v) = val_obj.get("value").and_then(serde_json::Value::as_f64)
                && v > 0.0
            {
                return Some(v);
            }
        }
        None
    }

    /// Helper to extract 7 crypto prices from a `DashboardData` instance.
    fn extract_prices_from_dashboard(data: &DashboardData) -> HashMap<String, CryptoPrice> {
        let mut prices = HashMap::with_capacity(7);
        prices.insert(
            "BTC".to_string(),
            CryptoPrice::new(data.btc_price_usd, data.btc_change_24h),
        );
        prices.insert(
            "ETH".to_string(),
            CryptoPrice::new(data.eth_price_usd, data.eth_change_24h),
        );
        prices.insert(
            "SOL".to_string(),
            CryptoPrice::new(data.sol_price_usd, data.sol_change_24h),
        );
        prices.insert(
            "XRP".to_string(),
            CryptoPrice::new(data.xrp_price_usd, data.xrp_change_24h),
        );
        prices.insert(
            "ADA".to_string(),
            CryptoPrice::new(data.ada_price_usd, data.ada_change_24h),
        );
        prices.insert(
            "LINK".to_string(),
            CryptoPrice::new(data.link_price_usd, data.link_change_24h),
        );
        prices.insert(
            "BNB".to_string(),
            CryptoPrice::new(data.bnb_price_usd, data.bnb_change_24h),
        );
        prices
    }

    /// Reads the latest `DashboardData` from Redis Stream (`market_data_stream`).
    async fn read_latest_dashboard_from_stream(&self) -> Result<DashboardData> {
        let entries = self
            .cache
            .cache_manager()
            .read_stream_latest("market_data_stream", 1)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read stream 'market_data_stream': {e}"))?;

        let (_entry_id, fields) = entries
            .first()
            .ok_or_else(|| anyhow::anyhow!("No entries found in market_data_stream"))?;

        // Case 1: Stream entry has a "data" field containing the JSON payload
        if let Some((_, value)) = fields.iter().find(|(k, _)| k == "data")
            && let Ok(data) = serde_json::from_str::<DashboardData>(value)
        {
            return Ok(data);
        }

        // Case 2: Stream fields are individual key-value pairs
        let mut map = serde_json::Map::new();
        for (k, v) in fields {
            if let Ok(parsed_val) = serde_json::from_str::<serde_json::Value>(v) {
                map.insert(k.clone(), parsed_val);
            } else {
                map.insert(k.clone(), serde_json::Value::String(v.clone()));
            }
        }
        let value = serde_json::Value::Object(map);
        serde_json::from_value::<DashboardData>(value)
            .context("Failed to deserialize DashboardData from Redis stream fields")
    }

    /// Checks the health status of market data components.
    #[must_use]
    pub fn health_check(&self) -> bool {
        self.binance_ws.is_healthy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_dashboard_data() -> DashboardData {
        DashboardData {
            btc_price_usd: 65000.0,
            btc_change_24h: 2.5,
            btc_market_cap_percentage: 54.0,
            btc_rsi_14: 60.5,
            eth_price_usd: 3500.0,
            eth_change_24h: 1.8,
            eth_market_cap_percentage: 16.5,
            sol_price_usd: 145.0,
            sol_change_24h: -0.5,
            xrp_price_usd: 0.55,
            xrp_change_24h: 0.1,
            ada_price_usd: 0.45,
            ada_change_24h: -1.2,
            link_price_usd: 15.0,
            link_change_24h: 3.4,
            bnb_price_usd: 580.0,
            bnb_change_24h: 0.8,
            market_cap_usd: 2_500_000_000_000.0,
            volume_24h_usd: 80_000_000_000.0,
            market_cap_change_percentage_24h_usd: 1.5,
            fng_value: 72,
            fetch_duration_ms: 120,
            partial_failure: false,
            last_updated: "2026-08-23T12:00:00Z".to_string(),
            timestamp: "2026-08-23T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_extract_prices_from_dashboard() {
        let data = mock_dashboard_data();
        let prices = MarketDataService::extract_prices_from_dashboard(&data);

        assert_eq!(prices.len(), 7);
        if let Some(btc) = prices.get("BTC") {
            assert!((btc.price_usd - 65000.0).abs() < f64::EPSILON);
            assert!((btc.change_24h - 2.5).abs() < f64::EPSILON);
        } else {
            panic!("BTC missing from extracted prices");
        }

        if let Some(eth) = prices.get("ETH") {
            assert!((eth.price_usd - 3500.0).abs() < f64::EPSILON);
        } else {
            panic!("ETH missing from extracted prices");
        }
    }

    #[test]
    fn test_stream_data_json_roundtrip() -> Result<()> {
        let original = mock_dashboard_data();
        let json_str = original.to_json_string()?;
        let parsed: DashboardData = serde_json::from_str(&json_str)?;

        assert!((parsed.btc_price_usd - original.btc_price_usd).abs() < f64::EPSILON);
        assert_eq!(parsed.fng_value, original.fng_value);
        assert!((parsed.btc_rsi_14 - original.btc_rsi_14).abs() < f64::EPSILON);
        assert!((parsed.market_cap_usd - original.market_cap_usd).abs() < f64::EPSILON);
        Ok(())
    }
}
