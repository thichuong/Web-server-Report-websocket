// Market Data API Core Component
//
// This module contains the core MarketDataApi struct and its constructor methods.

use reqwest::Client;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tracing::{info, warn, error};
use crate::performance::OPTIMIZED_HTTP_CLIENT;


/// Market Data API
///
/// Handles direct API calls to cryptocurrency data sources and stock market indices.
pub struct MarketDataApi {
    pub client: Client,
    pub taapi_secret: String,
    pub cmc_api_key: Option<String>,
    pub finnhub_api_key: Option<String>,
    // Statistics tracking
    pub api_calls_count: Arc<AtomicUsize>,
    pub successful_calls: Arc<AtomicUsize>,
    pub failed_calls: Arc<AtomicUsize>,
    pub last_call_timestamp: Arc<AtomicU64>,
}

impl MarketDataApi {
    /// Create a new MarketDataApi
    #[allow(dead_code)]
    pub async fn new(taapi_secret: String) -> Result<Self> {
        Self::with_cmc_key(taapi_secret, None).await
    }

    /// Create a new MarketDataApi with CoinMarketCap API key
    pub async fn with_cmc_key(taapi_secret: String, cmc_api_key: Option<String>) -> Result<Self> {
        Self::with_all_keys(taapi_secret, cmc_api_key, None).await
    }

    /// Create a new MarketDataApi with all API keys
    pub async fn with_all_keys(
        taapi_secret: String,
        cmc_api_key: Option<String>,
        finnhub_api_key: Option<String>
    ) -> Result<Self> {
        info!("Initializing Market Data API");

        // Use the optimized HTTP client from the performance module
        let client = OPTIMIZED_HTTP_CLIENT.clone();

        Ok(Self {
            client,
            taapi_secret,
            cmc_api_key,
            finnhub_api_key,
            api_calls_count: Arc::new(AtomicUsize::new(0)),
            successful_calls: Arc::new(AtomicUsize::new(0)),
            failed_calls: Arc::new(AtomicUsize::new(0)),
            last_call_timestamp: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Health check for Market Data API
    pub async fn health_check(&self) -> bool {
        match self.test_api_connectivity().await {
            Ok(_) => {
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
        let response = self.client
            .get("https://api.binance.com/api/v3/ping")
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("API connectivity test failed with status: {}", response.status()))
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
            Ordering::Relaxed
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