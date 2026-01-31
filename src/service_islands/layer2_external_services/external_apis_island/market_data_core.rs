// Market Data API Core Component
//
// This module contains the core MarketDataApi struct and its constructor methods.

use crate::performance::OPTIMIZED_HTTP_CLIENT;
use anyhow::Result;
use reqwest::Client;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Market Data API
///
/// Handles direct API calls to cryptocurrency data sources and stock market indices.
pub struct MarketDataApi {
    pub client: Client,
    pub taapi_secret: String,
    pub cmc_api_key: Option<String>,
    pub finnhub_api_key: Option<String>,
    pub binance_url: String,
    // Statistics tracking
    pub api_calls_count: Arc<AtomicUsize>,
    pub successful_calls: Arc<AtomicUsize>,
    pub failed_calls: Arc<AtomicUsize>,
    pub last_call_timestamp: Arc<AtomicU64>,
    pub last_cmc_call: Arc<AtomicU64>,
    pub last_rsi_call: Arc<AtomicU64>,
    pub last_finnhub_call: Arc<AtomicU64>,
    pub last_coingecko_call: Arc<AtomicU64>,
}

impl MarketDataApi {
    /// Create a new `MarketDataApi`
    ///
    /// # Errors
    /// Returns error if HTTP client initialization fails
    #[allow(dead_code)]
    pub async fn new(taapi_secret: String) -> Result<Self> {
        Self::with_cmc_key(taapi_secret, None).await
    }

    /// Create a new `MarketDataApi` with `CoinMarketCap` API key
    ///
    /// # Errors
    /// Returns error if HTTP client initialization fails
    pub async fn with_cmc_key(taapi_secret: String, cmc_api_key: Option<String>) -> Result<Self> {
        Self::with_all_keys(taapi_secret, cmc_api_key, None).await
    }

    /// Create a new `MarketDataApi` with all API keys
    ///
    /// # Errors
    /// Returns error if HTTP client initialization fails
    #[allow(clippy::unused_async)]
    pub async fn with_all_keys(
        taapi_secret: String,
        cmc_api_key: Option<String>,
        finnhub_api_key: Option<String>,
    ) -> Result<Self> {
        info!("Initializing Market Data API");

        // Use the optimized HTTP client from the performance module
        let client = OPTIMIZED_HTTP_CLIENT.clone();

        // Check Binance connectivity to decide which URL to use
        let binance_url = Self::check_binance_connectivity(&client).await;

        Ok(Self {
            client,
            taapi_secret,
            cmc_api_key,
            finnhub_api_key,
            binance_url,
            api_calls_count: Arc::new(AtomicUsize::new(0)),
            successful_calls: Arc::new(AtomicUsize::new(0)),
            failed_calls: Arc::new(AtomicUsize::new(0)),
            last_call_timestamp: Arc::new(AtomicU64::new(0)),
            last_cmc_call: Arc::new(AtomicU64::new(0)),
            last_rsi_call: Arc::new(AtomicU64::new(0)),
            last_finnhub_call: Arc::new(AtomicU64::new(0)),
            last_coingecko_call: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Check Binance connectivity and return the appropriate URL
    async fn check_binance_connectivity(client: &Client) -> String {
        info!("Checking Binance API connectivity...");

        // Try Global URL first
        // We use a simple ping or just check the multi-price URL with a HEAD request or simple GET
        // Using the actual endpoint we want to use is safer to detect 451
        let url = BINANCE_MULTI_PRICE_URL;

        match client.get(url).send().await {
            Ok(response) => {
                if response.status() == 451 {
                    warn!("Binance Global API returned 451 (Unavailable for legal reasons/Geoblocked). Switching to Binance US.");
                    BINANCE_US_MULTI_PRICE_URL.to_string()
                } else if response.status().is_success() {
                    info!("Binance Global API is accessible.");
                    BINANCE_MULTI_PRICE_URL.to_string()
                } else {
                    // Start up failed for other reasons, but we default to Global if it's not explicitly blocked
                    warn!(
                        "Binance Global API returned status: {}. Defaulting to Global URL.",
                        response.status()
                    );
                    BINANCE_MULTI_PRICE_URL.to_string()
                }
            }
            Err(e) => {
                warn!(
                    "Failed to connect to Binance Global API: {}. Checking Binance US...",
                    e
                );
                // If global fails completely (dns or connection refused), try US
                // If US also fails, we default to Global (or maybe US? Global seems safer default)
                let us_url = BINANCE_US_MULTI_PRICE_URL;
                match client.get(us_url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            info!("Binance US API is accessible. Switching to Binance US.");
                            BINANCE_US_MULTI_PRICE_URL.to_string()
                        } else {
                            warn!("Binance US API also returned status: {}. Defaulting to Global URL.", response.status());
                            BINANCE_MULTI_PRICE_URL.to_string()
                        }
                    }
                    Err(e_us) => {
                        error!("Failed to connect to Binance US API as well: {}. Defaulting to Global URL.", e_us);
                        BINANCE_MULTI_PRICE_URL.to_string()
                    }
                }
            }
        }
    }

    /// Health check for Market Data API
    pub async fn health_check(&self) -> bool {
        match self.test_api_connectivity().await {
            Ok(()) => {
                info!("Market Data API connectivity test passed");
                true
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("429") || error_str.contains("Too Many Requests") {
                    warn!("Market Data API health check: Rate limited, but service is available");
                    true // Rate limiting means API is working, just busy
                } else {
                    error!(error = %e, "Market Data API connectivity test failed");
                    false
                }
            }
        }
    }

    /// Test API connectivity
    async fn test_api_connectivity(&self) -> Result<()> {
        // Simple test call to Binance ping endpoint
        let response = self
            .client
            .get("https://api.binance.com/api/v3/ping")
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "API connectivity test failed with status: {}",
                response.status()
            ))
        }
    }

    /// Record an API call for statistics
    pub fn record_api_call(&self) {
        self.api_calls_count.fetch_add(1, Ordering::Relaxed);
        self.last_call_timestamp.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::from_secs(0))
                .as_secs(),
            Ordering::Relaxed,
        );
    }

    /// Record a successful API call
    pub fn record_success(&self) {
        self.successful_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed API call
    pub fn record_failure(&self) {
        self.failed_calls.fetch_add(1, Ordering::Relaxed);
    }
}
