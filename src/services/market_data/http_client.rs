//! Market Data HTTP Client Module
//!
//! Handles direct HTTP REST calls to cryptocurrency data sources:
//! - Binance REST (Multi-ticker fallback)
//! - `CoinGecko` (Global market data)
//! - `CoinMarketCap` (Global market data fallback)
//! - Alternative.me (Crypto Fear & Greed Index)
//! - `TAAPI.io` (Technical Analysis RSI-14)

use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::dto::websocket::CryptoPrice;

// ============================================================================
// API Endpoint URLs
// ============================================================================

pub const BINANCE_MULTI_PRICE_URL: &str = "https://api.binance.com/api/v3/ticker/24hr?symbols=%5B%22BTCUSDT%22,%22ETHUSDT%22,%22SOLUSDT%22,%22XRPUSDT%22,%22ADAUSDT%22,%22LINKUSDT%22,%22BNBUSDT%22%5D";
pub const BINANCE_US_MULTI_PRICE_URL: &str = "https://api.binance.us/api/v3/ticker/24hr?symbols=%5B%22BTCUSDT%22,%22ETHUSDT%22,%22SOLUSDT%22,%22XRPUSDT%22,%22ADAUSDT%22,%22LINKUSDT%22,%22BNBUSDT%22%5D";

pub const BINANCE_WS_STREAM_URL: &str = "wss://stream.binance.com:9443/stream?streams=btcusdt@ticker/ethusdt@ticker/solusdt@ticker/xrpusdt@ticker/adausdt@ticker/linkusdt@ticker/bnbusdt@ticker";
pub const BINANCE_US_WS_STREAM_URL: &str = "wss://stream.binance.us:9443/stream?streams=btcusdt@ticker/ethusdt@ticker/solusdt@ticker/xrpusdt@ticker/adausdt@ticker/linkusdt@ticker/bnbusdt@ticker";

pub const COINGECKO_GLOBAL_URL: &str = "https://api.coingecko.com/api/v3/global";
pub const CMC_GLOBAL_URL: &str =
    "https://pro-api.coinmarketcap.com/v1/global-metrics/quotes/latest";
pub const FNG_URL: &str = "https://api.alternative.me/fng/?limit=1";
pub const RSI_URL_TEMPLATE: &str =
    "https://api.taapi.io/rsi?secret={secret}&exchange=binance&symbol=BTC/USDT&interval=1d";

// ============================================================================
// Response Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMarketMetrics {
    pub market_cap_usd: f64,
    pub volume_24h_usd: f64,
    pub market_cap_change_percentage_24h_usd: f64,
    pub btc_market_cap_percentage: f64,
    pub eth_market_cap_percentage: f64,
}

#[derive(Debug, Deserialize)]
struct BinanceTickerItem {
    pub symbol: String,
    #[serde(rename = "lastPrice")]
    pub last_price: String,
    #[serde(rename = "priceChangePercent")]
    pub price_change_percent: String,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoGlobalResponse {
    data: CoinGeckoGlobalData,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoGlobalData {
    total_market_cap: HashMap<String, f64>,
    total_volume: HashMap<String, f64>,
    market_cap_change_percentage_24h_usd: f64,
    market_cap_percentage: HashMap<String, f64>,
}

#[derive(Debug, Deserialize)]
struct CmcGlobalResponse {
    data: CmcGlobalData,
}

#[derive(Debug, Deserialize)]
struct CmcGlobalData {
    quote: HashMap<String, CmcGlobalQuote>,
}

#[derive(Debug, Deserialize)]
struct CmcGlobalQuote {
    total_market_cap: f64,
    total_volume_24h: f64,
    market_cap_change_percentage_24h: f64,
    btc_dominance: f64,
    eth_dominance: f64,
}

#[derive(Debug, Deserialize)]
struct FearGreedResponse {
    data: Vec<FearGreedItem>,
}

#[derive(Debug, Deserialize)]
struct FearGreedItem {
    value: String,
}

#[derive(Debug, Deserialize)]
struct TaapiRsiResponse {
    value: f64,
}

// ============================================================================
// HTTP Client Implementation
// ============================================================================

/// HTTP Client for interacting with external cryptocurrency REST APIs.
pub struct MarketDataHttpClient {
    client: Client,
}

impl MarketDataHttpClient {
    /// Creates a new `MarketDataHttpClient` using the shared optimized HTTP client.
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Automatically detects Binance API availability (Global vs US fallback if geoblocked).
    pub async fn check_binance_connectivity(&self) -> &'static str {
        info!("Checking Binance API connectivity...");
        match self.client.get(BINANCE_MULTI_PRICE_URL).send().await {
            Ok(response) => {
                if response.status() == 451 {
                    warn!("Binance Global API returned 451 (Geoblocked). Switching to Binance US.");
                    BINANCE_US_MULTI_PRICE_URL
                } else if response.status().is_success() {
                    info!("Binance Global API is accessible.");
                    BINANCE_MULTI_PRICE_URL
                } else {
                    warn!(
                        "Binance Global API returned status: {}. Defaulting to Global.",
                        response.status()
                    );
                    BINANCE_MULTI_PRICE_URL
                }
            }
            Err(e) => {
                warn!("Failed to connect to Binance Global API: {e}. Testing Binance US...");
                match self.client.get(BINANCE_US_MULTI_PRICE_URL).send().await {
                    Ok(res) if res.status().is_success() => {
                        info!("Binance US API is accessible. Using Binance US.");
                        BINANCE_US_MULTI_PRICE_URL
                    }
                    _ => {
                        warn!("Binance US API inaccessible, defaulting to Global URL.");
                        BINANCE_MULTI_PRICE_URL
                    }
                }
            }
        }
    }

    /// Fetches 7 crypto ticker prices via Binance REST API.
    ///
    /// # Errors
    /// Returns an error if the request fails, or if parsing/validation fails.
    pub async fn fetch_multi_crypto_prices(
        &self,
        url: &str,
    ) -> Result<HashMap<String, CryptoPrice>> {
        let response = self
            .client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to send request to Binance multi-ticker endpoint")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Binance API returned status: {}",
                response.status()
            ));
        }

        let tickers: Vec<BinanceTickerItem> = response
            .json()
            .await
            .context("Failed to parse Binance ticker response JSON")?;

        let mut prices = HashMap::with_capacity(7);
        for ticker in tickers {
            let coin_name = match ticker.symbol.as_str() {
                "BTCUSDT" => "BTC",
                "ETHUSDT" => "ETH",
                "SOLUSDT" => "SOL",
                "XRPUSDT" => "XRP",
                "ADAUSDT" => "ADA",
                "LINKUSDT" => "LINK",
                "BNBUSDT" => "BNB",
                _ => continue,
            };

            let price_usd = ticker.last_price.parse::<f64>().unwrap_or(0.0);
            let change_24h = ticker.price_change_percent.parse::<f64>().unwrap_or(0.0);

            if price_usd > 0.0 {
                prices.insert(
                    coin_name.to_string(),
                    CryptoPrice::new(price_usd, change_24h),
                );
            }
        }

        if prices.len() < 7 {
            return Err(anyhow!(
                "Binance returned incomplete tickers (expected 7, got {})",
                prices.len()
            ));
        }

        Ok(prices)
    }

    /// Fetches global market data from `CoinGecko`.
    ///
    /// # Errors
    /// Returns an error if the request fails or if validation fails.
    pub async fn fetch_global_coingecko(&self) -> Result<GlobalMarketMetrics> {
        let response = self
            .client
            .get(COINGECKO_GLOBAL_URL)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to send request to CoinGecko global endpoint")?;

        if !response.status().is_success() {
            return Err(anyhow!("CoinGecko returned status: {}", response.status()));
        }

        let data: CoinGeckoGlobalResponse = response
            .json()
            .await
            .context("Failed to parse CoinGecko global response JSON")?;

        let market_cap = data
            .data
            .total_market_cap
            .get("usd")
            .copied()
            .unwrap_or(0.0);
        let volume_24h = data.data.total_volume.get("usd").copied().unwrap_or(0.0);
        let btc_dominance = data
            .data
            .market_cap_percentage
            .get("btc")
            .copied()
            .unwrap_or(0.0);
        let eth_dominance = data
            .data
            .market_cap_percentage
            .get("eth")
            .copied()
            .unwrap_or(0.0);

        if market_cap <= 0.0 || volume_24h <= 0.0 || btc_dominance <= 0.0 {
            return Err(anyhow!(
                "CoinGecko data validation failed: zero or negative metrics"
            ));
        }

        Ok(GlobalMarketMetrics {
            market_cap_usd: market_cap,
            volume_24h_usd: volume_24h,
            market_cap_change_percentage_24h_usd: data.data.market_cap_change_percentage_24h_usd,
            btc_market_cap_percentage: btc_dominance,
            eth_market_cap_percentage: eth_dominance,
        })
    }

    /// Fetches global market data from `CoinMarketCap` (fallback).
    ///
    /// # Errors
    /// Returns an error if `CoinMarketCap` API key is missing or request fails.
    pub async fn fetch_global_cmc(&self, cmc_api_key: &str) -> Result<GlobalMarketMetrics> {
        let response = self
            .client
            .get(CMC_GLOBAL_URL)
            .header("X-CMC_PRO_API_KEY", cmc_api_key)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to send request to CoinMarketCap global endpoint")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "CoinMarketCap returned status: {}",
                response.status()
            ));
        }

        let cmc_data: CmcGlobalResponse = response
            .json()
            .await
            .context("Failed to parse CoinMarketCap response JSON")?;

        let quote = cmc_data
            .data
            .quote
            .get("USD")
            .ok_or_else(|| anyhow!("Missing USD quote in CoinMarketCap response"))?;

        Ok(GlobalMarketMetrics {
            market_cap_usd: quote.total_market_cap,
            volume_24h_usd: quote.total_volume_24h,
            market_cap_change_percentage_24h_usd: quote.market_cap_change_percentage_24h,
            btc_market_cap_percentage: quote.btc_dominance,
            eth_market_cap_percentage: quote.eth_dominance,
        })
    }

    /// Fetches Crypto Fear & Greed Index from Alternative.me.
    ///
    /// # Errors
    /// Returns an error if the request fails or value cannot be parsed.
    pub async fn fetch_fear_greed(&self) -> Result<u32> {
        let response = self
            .client
            .get(FNG_URL)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to send request to Fear & Greed endpoint")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Fear & Greed API returned status: {}",
                response.status()
            ));
        }

        let fng_data: FearGreedResponse = response
            .json()
            .await
            .context("Failed to parse Fear & Greed response JSON")?;

        let value_str = fng_data
            .data
            .first()
            .map_or("50", |item| item.value.as_str());

        let fng_value = value_str.parse::<u32>().unwrap_or(50);
        Ok(fng_value)
    }

    /// Fetches BTC 14-day RSI from `TAAPI.io`.
    ///
    /// # Errors
    /// Returns an error if the request fails or value cannot be parsed.
    pub async fn fetch_btc_rsi_14(&self, secret: &str) -> Result<f64> {
        let url = RSI_URL_TEMPLATE.replace("{secret}", secret);
        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to send request to TAAPI RSI endpoint")?;

        if !response.status().is_success() {
            return Err(anyhow!("TAAPI RSI returned status: {}", response.status()));
        }

        let rsi_data: TaapiRsiResponse = response
            .json()
            .await
            .context("Failed to parse TAAPI RSI response JSON")?;

        Ok(rsi_data.value)
    }

    /// Tests basic API connectivity for health checks.
    ///
    /// # Errors
    /// Returns an error if ping fails.
    pub async fn test_connectivity(&self) -> Result<()> {
        let response = self
            .client
            .get("https://api.binance.com/api/v3/ping")
            .send()
            .await
            .context("Failed to ping Binance API")?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Ping returned non-success status: {}",
                response.status()
            ))
        }
    }
}
