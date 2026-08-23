//! Market Data Service Module
//!
//! Orchestrates cryptocurrency data collection from real-time WebSocket feeds,
//! external HTTP APIs, and the multi-tier caching system to produce unified `DashboardData`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::Result;
use chrono::Utc;
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
            warn!("⚠️ TAAPI_SECRET is not configured or using default - BTC RSI-14 will default to neutral 50.0");
        }
        if cmc_api_key.is_none() {
            warn!("⚠️ CMC_API_KEY is not configured - CoinMarketCap fallback for global market data is disabled");
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
            warn!(duration_ms = duration.as_millis(), "Dashboard data aggregated with partial failures");
        } else {
            info!(duration_ms = duration.as_millis(), "Dashboard data aggregated successfully");
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

    /// Fetches 7 crypto prices using in-memory WebSocket feed first, with HTTP REST fallback and caching.
    async fn fetch_crypto_prices(&self, force_refresh: bool) -> Result<HashMap<String, CryptoPrice>> {
        // Fast path: In-memory real-time WebSocket cache (<30s fresh)
        if !force_refresh {
            if let Some(ws_prices) = self.binance_ws.get_multi_crypto_prices() {
                debug!("Obtained fresh real-time crypto prices from Binance WebSocket cache");
                return Ok(ws_prices);
            }
            debug!("WebSocket cache stale or incomplete, falling back to cached/REST endpoint");
        }

        let cache_key = "multi_crypto_prices_realtime";

        if force_refresh {
            let prices = self
                .http_client
                .fetch_multi_crypto_prices(&self.binance_rest_url)
                .await?;
            self.binance_ws.seed_prices(&prices);

            if let Ok(vec) = serde_json::to_vec(&prices) {
                let _ = self
                    .cache
                    .cache_manager()
                    .set_with_strategy(cache_key, multi_tier_cache::Bytes::from(vec), realtime_strategy())
                    .await;
            }
            return Ok(prices);
        }

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
    }

    /// Fetches global market data with 1-hour caching and CMC fallback.
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

        result.map_err(anyhow::Error::from)
    }

    /// Fetches Crypto Fear & Greed Index with 5-minute caching.
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

        result.map_err(anyhow::Error::from)
    }

    /// Fetches BTC 14-day RSI with 3-hour caching.
    async fn fetch_rsi(&self, _force_refresh: bool) -> Result<f64> {
        if self.taapi_secret.is_empty() || self.taapi_secret == "default_secret" {
            debug!("TAAPI secret not configured, using neutral RSI (50.0)");
            return Ok(50.0);
        }

        let http_client = Arc::clone(&self.http_client);
        let secret = self.taapi_secret.clone();

        let result = self
            .cache
            .cache_manager()
            .get_or_compute_typed(
                "btc_rsi_14_taapi_3h",
                CacheStrategy::LongTerm,
                || async move {
                    http_client
                        .fetch_btc_rsi_14(&secret)
                        .await
                        .map_err(|e| multi_tier_cache::CacheError::BackendError(e.to_string()))
                },
            )
            .await;

        result.map_err(anyhow::Error::from)
    }

    /// Checks the health status of market data components.
    #[must_use]
    pub fn health_check(&self) -> bool {
        self.binance_ws.is_healthy()
    }
}
