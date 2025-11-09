// Market Data Fetchers Component
//
// This module contains market data fetching methods for global data, FNG, RSI, and US indices.

use futures;

impl MarketDataApi {
    /// Fetch global market data with fallback chain
    pub async fn fetch_global_data(&self) -> Result<serde_json::Value> {
        self.record_api_call();

        // Try CoinGecko first
        match self.fetch_global_data_coingecko().await {
            Ok(data) => {
                self.record_success();
                Ok(data)
            }
            Err(e) => {
                println!("⚠️ CoinGecko global data failed: {}, trying CoinMarketCap...", e);
                // Fallback to CoinMarketCap
                match self.fetch_global_data_cmc().await {
                    Ok(data) => {
                        self.record_success();
                        Ok(data)
                    }
                    Err(cmc_err) => {
                        self.record_failure();
                        println!("❌ Both CoinGecko and CoinMarketCap failed for global data");
                        Err(anyhow::anyhow!("Primary error: {}. Fallback error: {}", e, cmc_err))
                    }
                }
            }
        }
    }

    /// Fetch global data from CoinGecko
    async fn fetch_global_data_coingecko(&self) -> Result<serde_json::Value> {
        let result = self.fetch_with_retry(BASE_GLOBAL_URL, |global_data: CoinGeckoGlobal| {
            let market_cap = global_data.data.total_market_cap.get("usd").copied().unwrap_or(0.0);
            let volume_24h = global_data.data.total_volume.get("usd").copied().unwrap_or(0.0);
            let market_cap_change_24h = global_data.data.market_cap_change_percentage_24h_usd;
            let btc_dominance = global_data.data.market_cap_percentage.get("btc").copied().unwrap_or(0.0);
            let eth_dominance = global_data.data.market_cap_percentage.get("eth").copied().unwrap_or(0.0);

            serde_json::json!({
                "market_cap": market_cap,
                "volume_24h": volume_24h,
                "market_cap_change_percentage_24h_usd": market_cap_change_24h,
                "btc_market_cap_percentage": btc_dominance,
                "eth_market_cap_percentage": eth_dominance,
                "source": "coingecko",
                "last_updated": chrono::Utc::now().to_rfc3339()
            })
        }).await?;

        // Post-processing validation: check if we got meaningful data
        let market_cap = result.get("market_cap").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let volume_24h = result.get("volume_24h").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let btc_dominance = result.get("btc_market_cap_percentage").and_then(|v| v.as_f64()).unwrap_or(0.0);

        // Critical validation: if any essential data is missing or invalid, return error
        if market_cap <= 0.0 || volume_24h <= 0.0 || btc_dominance <= 0.0 {
            return Err(anyhow::anyhow!(
                "CoinGecko data validation failed: market_cap={}, volume_24h={}, btc_dominance={}",
                market_cap, volume_24h, btc_dominance
            ));
        }

        Ok(result)
    }

    /// Fetch global data from CoinMarketCap
    async fn fetch_global_data_cmc(&self) -> Result<serde_json::Value> {
        let cmc_key = self.cmc_api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("CoinMarketCap API key not provided"))?;

        let mut attempts = 0;
        let max_attempts = 3;

        while attempts < max_attempts {
            let response = self.client
                .get(CMC_GLOBAL_URL)
                .header("X-CMC_PRO_API_KEY", cmc_key)
                .header("Accept", "application/json")
                .send()
                .await?;

            match response.status() {
                status if status.is_success() => {
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
                    return Err(anyhow::anyhow!("Invalid CoinMarketCap global response structure"));
                }
                status if status == 429 => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow::anyhow!("CoinMarketCap global API rate limit exceeded after {} attempts", max_attempts));
                    }

                    let delay = std::time::Duration::from_millis(1000 * (2_u64.pow(attempts)));
                    println!("⚠️ CoinMarketCap global API rate limit (429), retrying in {:?} (attempt {}/{})", delay, attempts, max_attempts);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                status => {
                    return Err(anyhow::anyhow!("CoinMarketCap global API returned status: {}", status));
                }
            }
        }

        Err(anyhow::anyhow!("CoinMarketCap global API max retry attempts reached"))
    }

    /// Fetch Fear & Greed Index
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
        }).await
    }

    /// Fetch RSI data
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
    async fn fetch_btc_rsi_14_internal(&self) -> Result<serde_json::Value> {
        let url = BASE_RSI_URL_TEMPLATE.replace("{secret}", &self.taapi_secret);

        // RSI uses a different approach because URL is dynamic
        let mut attempts = 0;
        let max_attempts = 3;

        while attempts < max_attempts {
            let response = self.client
                .get(&url)
                .send()
                .await?;

            match response.status() {
                status if status.is_success() => {
                    let btc_rsi_14_data: TaapiRsiResponse = response.json().await?;
                    return Ok(serde_json::json!({
                        "value": btc_rsi_14_data.value,
                        "period": "14",
                        "last_updated": chrono::Utc::now().to_rfc3339()
                    }));
                }
                status if status == 429 => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow::anyhow!("RSI API rate limit exceeded after {} attempts", max_attempts));
                    }

                    let delay = std::time::Duration::from_millis(1000 * (2_u64.pow(attempts)));
                    println!("⚠️ RSI API rate limit (429), retrying in {:?} (attempt {}/{})", delay, attempts, max_attempts);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                status => {
                    return Err(anyhow::anyhow!("RSI API returned status: {}", status));
                }
            }
        }

        Err(anyhow::anyhow!("RSI API max retry attempts reached"))
    }

    /// Fetch US Stock Market Indices from Finnhub
    pub async fn fetch_us_stock_indices(&self) -> Result<serde_json::Value> {
        self.record_api_call();

        match self.fetch_us_indices_internal().await {
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

    /// Internal US stock indices fetching
    async fn fetch_us_indices_internal(&self) -> Result<serde_json::Value> {
        let finnhub_key = self.finnhub_api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Finnhub API key not provided"))?;

        // Define the indices we want to fetch (using ETFs as proxies for free tier)
        let indices = vec![
            ("DIA", "SPDR Dow Jones Industrial Average ETF"),  // DJIA proxy
            ("SPY", "SPDR S&P 500 ETF Trust"),                // S&P 500 proxy
            ("QQQM", "INVESCO NASDAQ 100 ETF"),                      // Nasdaq 100 proxy
        ];

        let mut results = HashMap::new();
        let mut all_success = true;

        // Fetch each index concurrently
        let futures: Vec<_> = indices.iter().map(|(symbol, name)| {
            self.fetch_single_index(symbol, name, finnhub_key)
        }).collect();

        let index_results = futures::future::join_all(futures).await;

        // Process results
        for (i, result) in index_results.into_iter().enumerate() {
            let (symbol, name) = &indices[i];
            match result {
                Ok(index_data) => {
                    results.insert(symbol.to_string(), index_data);
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to fetch {}: {}", name, e);
                    all_success = false;
                    // Insert placeholder data for failed fetch
                    results.insert(symbol.to_string(), serde_json::json!({
                        "symbol": symbol,
                        "name": name,
                        "price": 0.0,
                        "change": 0.0,
                        "change_percent": 0.0,
                        "status": "failed"
                    }));
                }
            }
        }

        if !all_success {
            return Err(anyhow::anyhow!("Some US indices failed to fetch"));
        }

        Ok(serde_json::json!({
            "indices": results,
            "source": "finnhub",
            "last_updated": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Fetch single index from Finnhub
    async fn fetch_single_index(&self, symbol: &str, name: &str, api_key: &str) -> Result<serde_json::Value> {
        let url = format!("https://finnhub.io/api/v1/quote?symbol={}&token={}", symbol, api_key);

        let mut attempts = 0;
        let max_attempts = 3;

        while attempts < max_attempts {
            let response = self.client
                .get(&url)
                .send()
                .await?;

            match response.status() {
                status if status.is_success() => {
                    let finnhub_data: FinnhubQuoteResponse = response.json().await?;

                    // Validate data
                    if finnhub_data.current_price <= 0.0 {
                        return Err(anyhow::anyhow!("Invalid price data for {}: {}", symbol, finnhub_data.current_price));
                    }

                    return Ok(serde_json::json!({
                        "symbol": symbol,
                        "name": name,
                        "price": finnhub_data.current_price,
                        "change": finnhub_data.change,
                        "change_percent": finnhub_data.percent_change,
                        "status": "success"
                    }));
                }
                status if status == 429 => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow::anyhow!("Finnhub rate limit exceeded for {} after {} attempts", symbol, max_attempts));
                    }

                    let delay = std::time::Duration::from_millis(1000 * (2_u64.pow(attempts)));
                    println!("⚠️ Finnhub rate limit (429) for {}, retrying in {:?} (attempt {}/{})", symbol, delay, attempts, max_attempts);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                status => {
                    return Err(anyhow::anyhow!("Finnhub API returned status {} for {}", status, symbol));
                }
            }
        }

        Err(anyhow::anyhow!("Finnhub API max retry attempts reached for {}", symbol))
    }

    /// Get API statistics
    #[allow(dead_code)]
    pub fn get_api_stats(&self) -> serde_json::Value {
        let total_calls = self.api_calls_count.load(std::sync::atomic::Ordering::Relaxed);
        let successful_calls = self.successful_calls.load(std::sync::atomic::Ordering::Relaxed);
        let failed_calls = self.failed_calls.load(std::sync::atomic::Ordering::Relaxed);
        let last_call = self.last_call_timestamp.load(std::sync::atomic::Ordering::Relaxed);

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
            "has_coinmarketcap_key": self.cmc_api_key.is_some(),
            "has_finnhub_key": self.finnhub_api_key.is_some()
        })
    }
}