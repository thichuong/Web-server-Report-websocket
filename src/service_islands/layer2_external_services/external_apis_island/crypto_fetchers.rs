// Cryptocurrency Price Fetchers Component
//
// This module contains all cryptocurrency price fetching methods with fallback logic.

impl MarketDataApi {
    /// Fetch multiple crypto prices in a single Binance API call (OPTIMIZED)
    /// 
    /// Fetches BTC, ETH, SOL, XRP, ADA, LINK, BNB prices in one request
    /// Returns HashMap<Symbol, (price_usd, change_24h)>
    pub async fn fetch_multi_crypto_prices(&self) -> Result<HashMap<String, (f64, f64)>> {
        self.record_api_call();

        // Try Binance multi-ticker endpoint
        match self.fetch_multi_crypto_prices_binance().await {
            Ok(data) => {
                self.record_success();
                Ok(data)
            }
            Err(e) => {
                self.record_failure();
                println!("❌ Binance multi-ticker failed: {}", e);
                Err(e)
            }
        }
    }

    /// Fetch multiple crypto prices from Binance using multi-symbol endpoint
    async fn fetch_multi_crypto_prices_binance(&self) -> Result<HashMap<String, (f64, f64)>> {
        let response_json = self.fetch_with_retry(
            BINANCE_MULTI_PRICE_URL,
            |response_data: BinanceMultiTickerResponse| {
                // Just convert the vec to JSON
                serde_json::to_value(&response_data).unwrap_or(serde_json::json!([]))
            }
        ).await?;

        // Parse the JSON array into our HashMap
        let tickers: BinanceMultiTickerResponse = serde_json::from_value(response_json)?;
        let mut prices = HashMap::new();
        
        for ticker in tickers {
            let price_usd: f64 = ticker.last_price.parse().unwrap_or(0.0);
            let change_24h: f64 = ticker.price_change_percent.parse().unwrap_or(0.0);
            
            // Map symbol to coin name
            let coin_name = match ticker.symbol.as_str() {
                "BTCUSDT" => "BTC",
                "ETHUSDT" => "ETH",
                "SOLUSDT" => "SOL",
                "XRPUSDT" => "XRP",
                "ADAUSDT" => "ADA",
                "LINKUSDT" => "LINK",
                "BNBUSDT" => "BNB",
                _ => continue, // Skip unknown symbols
            };
            
            prices.insert(coin_name.to_string(), (price_usd, change_24h));
        }

        // Validate we got all 7 coins
        if prices.len() != 7 {
            return Err(anyhow::anyhow!(
                "Binance multi-ticker validation failed: expected 7 coins, got {}",
                prices.len()
            ));
        }

        // Validate each price is reasonable
        for (coin, (price, _)) in &prices {
            if *price <= 0.0 {
                return Err(anyhow::anyhow!(
                    "Binance {} price validation failed: price={}",
                    coin, price
                ));
            }
        }

        Ok(prices)
    }

    /// Generic fetch with retry logic and exponential backoff
    pub async fn fetch_with_retry<T, F>(&self, url: &str, transformer: F) -> Result<serde_json::Value>
    where
        T: for<'de> serde::Deserialize<'de>,
        F: Fn(T) -> serde_json::Value,
    {
        let mut attempts = 0;
        let max_attempts = 3;

        while attempts < max_attempts {
            let response = self.client
                .get(url)
                .header("Accept", "application/json")
                .send()
                .await?;

            match response.status() {
                status if status.is_success() => {
                    let data: T = response.json().await?;
                    return Ok(transformer(data));
                }
                status if status == 418 => {
                    // 418 I'm a teapot - Binance uses this for rate limiting/blocking
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow::anyhow!("Binance blocked request (418 I'm a teapot) after {} attempts for URL: {}. This usually means rate limiting or IP blocking.", max_attempts, url));
                    }

                    let delay = std::time::Duration::from_millis(2000 * (2_u64.pow(attempts)));
                    println!("⚠️ Binance blocking (418) for {}, retrying in {:?} (attempt {}/{})", url, delay, attempts, max_attempts);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                status if status == 429 => {
                    // Rate limiting - implement exponential backoff
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow::anyhow!("Rate limit exceeded after {} attempts for URL: {}", max_attempts, url));
                    }

                    let delay = std::time::Duration::from_millis(1000 * (2_u64.pow(attempts)));
                    println!("⚠️ Rate limit (429) hit for {}, retrying in {:?} (attempt {}/{})", url, delay, attempts, max_attempts);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                status => {
                    return Err(anyhow::anyhow!("API returned status: {} for URL: {}", status, url));
                }
            }
        }

        Err(anyhow::anyhow!("Max retry attempts reached for URL: {}", url))
    }
}