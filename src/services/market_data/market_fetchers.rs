// Market Data Fetchers Component
//
// This module contains market data fetching methods for global data, FNG, RSI, and US indices.

impl MarketDataApi {
    /// Fetch global market data with fallback chain
    ///
    /// # Errors
    /// Returns error if all API sources (`CoinGecko` and `CoinMarketCap`) fail or validation fails
    pub async fn fetch_global_data(&self) -> Result<serde_json::Value> {
        self.record_api_call();

        // Try CoinGecko first
        match self.fetch_global_data_coingecko().await {
            Ok(data) => {
                self.record_success();
                Ok(data)
            }
            Err(e) => {
                warn!(error = %e, "CoinGecko global data failed, trying CoinMarketCap");
                // Fallback to CoinMarketCap
                match self.fetch_global_data_cmc().await {
                    Ok(data) => {
                        self.record_success();
                        Ok(data)
                    }
                    Err(cmc_err) => {
                        self.record_failure();
                        error!("Both CoinGecko and CoinMarketCap failed for global data");
                        Err(anyhow::anyhow!(
                            "Primary error: {e}. Fallback error: {cmc_err}"
                        ))
                    }
                }
            }
        }
    }

    /// Fetch global data from `CoinGecko`
    ///
    /// Implements strict rate limiting (30s interval).
    async fn fetch_global_data_coingecko(&self) -> Result<serde_json::Value> {
        // 1. Independent Time Check
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs();

        let last_call = self
            .last_coingecko_call
            .load(std::sync::atomic::Ordering::Relaxed);

        if now < last_call + 30 {
            return Err(anyhow::anyhow!(
                "Skipped: CoinGecko rate limit precaution (Wait 30s)"
            ));
        }

        // Update timestamp immediately to enforce interval
        self.last_coingecko_call
            .store(now, std::sync::atomic::Ordering::Relaxed);

        let result = self
            .fetch_with_retry(BASE_GLOBAL_URL, |global_data: CoinGeckoGlobal| {
                let market_cap = global_data
                    .data
                    .total_market_cap
                    .get("usd")
                    .copied()
                    .unwrap_or(0.0);
                let volume_24h = global_data
                    .data
                    .total_volume
                    .get("usd")
                    .copied()
                    .unwrap_or(0.0);
                let market_cap_change_24h = global_data.data.market_cap_change_percentage_24h_usd;
                let btc_dominance = global_data
                    .data
                    .market_cap_percentage
                    .get("btc")
                    .copied()
                    .unwrap_or(0.0);
                let eth_dominance = global_data
                    .data
                    .market_cap_percentage
                    .get("eth")
                    .copied()
                    .unwrap_or(0.0);

                serde_json::json!({
                    "market_cap": market_cap,
                    "volume_24h": volume_24h,
                    "market_cap_change_percentage_24h_usd": market_cap_change_24h,
                    "btc_market_cap_percentage": btc_dominance,
                    "eth_market_cap_percentage": eth_dominance,
                    "source": "coingecko",
                    "last_updated": chrono::Utc::now().to_rfc3339()
                })
            })
            .await?;

        // Post-processing validation: check if we got meaningful data
        let market_cap = result
            .get("market_cap")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let volume_24h = result
            .get("volume_24h")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let btc_dominance = result
            .get("btc_market_cap_percentage")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);

        // Critical validation: if any essential data is missing or invalid, return error
        if market_cap <= 0.0 || volume_24h <= 0.0 || btc_dominance <= 0.0 {
            return Err(anyhow::anyhow!(
                "CoinGecko data validation failed: market_cap={market_cap}, volume_24h={volume_24h}, btc_dominance={btc_dominance}"
            ));
        }

        Ok(result)
    }

    /// Fetch global data from `CoinMarketCap`
    ///
    /// Implements strict rate limiting (60s interval) and no retries.
    async fn fetch_global_data_cmc(&self) -> Result<serde_json::Value> {
        let cmc_key = self
            .cmc_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CoinMarketCap API key not provided"))?;

        // 1. Independent Time Check
        // Prevent calling API if less than 60 seconds have passed since last call
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs();

        let last_call = self
            .last_cmc_call
            .load(std::sync::atomic::Ordering::Relaxed);

        if now < last_call + 60 {
            // Signal to use fallback/cache (by returning error, assumed handled by caller)
            return Err(anyhow::anyhow!(
                "Skipped: CMC rate limit precaution (Wait 60s)"
            ));
        }

        // 2. No Retry Logic - Single Attempt
        let response = self
            .client
            .get(CMC_GLOBAL_URL)
            .header("X-CMC_PRO_API_KEY", cmc_key)
            .header("Accept", "application/json")
            .send()
            .await?;

        if response.status().is_success() {
            // Update timestamp only on success
            self.last_cmc_call
                .store(now, std::sync::atomic::Ordering::Relaxed);

            let cmc_data: CmcGlobalResponse = response.json().await?;

            if let Some(usd_quote) = cmc_data.data.quote.get("USD") {
                return Ok(serde_json::json!({
                    "market_cap": usd_quote.total_market_cap,
                    "volume_24h": usd_quote.total_volume_24h,
                    "market_cap_change_percentage_24h_usd": usd_quote.market_cap_change_percentage_24h,
                    "btc_market_cap_percentage": usd_quote.btc_dominance,
                    "eth_market_cap_percentage": usd_quote.eth_dominance,
                    "source": "coinmarketcap",
                    "last_updated": chrono::Utc::now().to_rfc3339()
                }));
            }
            return Err(anyhow::anyhow!(
                "Invalid CoinMarketCap global response structure"
            ));
        } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // Update timestamp to prevent immediate retry even if failed
            self.last_cmc_call
                .store(now, std::sync::atomic::Ordering::Relaxed);
            warn!("CoinMarketCap 429 Rate Limit Hit - Backing off");
        }

        Err(anyhow::anyhow!(
            "CoinMarketCap global API failed with status: {}",
            response.status()
        ))
    }

    /// Fetch Fear & Greed Index
    ///
    /// # Errors
    /// Returns error if API fetch fails or response parsing fails
    pub async fn fetch_fear_greed_index(&self) -> Result<serde_json::Value> {
        self.record_api_call();

        match self.fetch_fear_greed_internal().await {
            Ok(data) => {
                self.record_success();
                Ok(data)
            }
            Err(e) => {
                self.record_failure();
                Err(e)
            }
        }
    }

    /// Internal Fear & Greed fetching
    async fn fetch_fear_greed_internal(&self) -> Result<serde_json::Value> {
        self.fetch_with_retry(BASE_FNG_URL, |fng_data: FearGreedResponse| {
            let fng_value: u32 = fng_data
                .data
                .first()
                .and_then(|d| d.value.parse().ok())
                .unwrap_or(50); // Default neutral value

            serde_json::json!({
                "value": fng_value,
                "last_updated": chrono::Utc::now().to_rfc3339()
            })
        })
        .await
    }

    /// Fetch RSI data
    ///
    /// # Errors
    /// Returns error if API fetch fails, rate limit is exceeded, or response parsing fails
    pub async fn fetch_btc_rsi_14(&self) -> Result<serde_json::Value> {
        self.record_api_call();

        match self.fetch_btc_rsi_14_internal().await {
            Ok(data) => {
                self.record_success();
                Ok(data)
            }
            Err(e) => {
                self.record_failure();
                Err(e)
            }
        }
    }

    /// Internal RSI fetching
    ///
    /// Implements strict rate limiting (120s interval) and no retries.
    async fn fetch_btc_rsi_14_internal(&self) -> Result<serde_json::Value> {
        let url = BASE_RSI_URL_TEMPLATE.replace("{secret}", &self.taapi_secret);

        // 1. Independent Time Check
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs();

        let last_call = self
            .last_rsi_call
            .load(std::sync::atomic::Ordering::Relaxed);

        if now < last_call + 120 {
            return Err(anyhow::anyhow!(
                "Skipped: RSI rate limit precaution (Wait 120s)"
            ));
        }

        // 2. No Retry Logic - Single Attempt
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            self.last_rsi_call
                .store(now, std::sync::atomic::Ordering::Relaxed);
            let btc_rsi_14_data: TaapiRsiResponse = response.json().await?;
            return Ok(serde_json::json!({
                "value": btc_rsi_14_data.value,
                "period": "14",
                "last_updated": chrono::Utc::now().to_rfc3339()
            }));
        } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            self.last_rsi_call
                .store(now, std::sync::atomic::Ordering::Relaxed);
            warn!("RSI API 429 Rate Limit Hit - Backing off");
        }

        Err(anyhow::anyhow!(
            "RSI API failed with status: {}",
            response.status()
        ))
    }

    /// Get API statistics
    #[must_use]
    #[allow(dead_code)]
    #[allow(clippy::cast_precision_loss)]
    pub fn get_api_stats(&self) -> serde_json::Value {
        let total_calls = self
            .api_calls_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let successful_calls = self
            .successful_calls
            .load(std::sync::atomic::Ordering::Relaxed);
        let failed_calls = self.failed_calls.load(std::sync::atomic::Ordering::Relaxed);
        let last_call = self
            .last_call_timestamp
            .load(std::sync::atomic::Ordering::Relaxed);

        serde_json::json!({
            "total_api_calls": total_calls,
            "successful_calls": successful_calls,
            "failed_calls": failed_calls,
            "success_rate": if total_calls > 0 {
                (successful_calls as f64 / total_calls as f64 * 100.0).round()
            } else {
                0.0
            },
            "last_call_timestamp": last_call,
            "has_coinmarketcap_key": self.cmc_api_key.is_some()
        })
    }
}
