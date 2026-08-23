//! Binance WebSocket Client
//!
//! Connects to Binance Combined WebSocket streams for real-time 24hr ticker updates.
//! Stores current ticker prices in-memory and provides fast, low-latency access for dashboard aggregation.
//! Automatically reconnects with exponential backoff and fails over to Binance US if necessary.

use dashmap::DashMap;
use futures::StreamExt;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

use super::http_client::{BINANCE_US_WS_STREAM_URL, BINANCE_WS_STREAM_URL};
use crate::dto::websocket::CryptoPrice;

const TRACKED_COINS: [&str; 7] = ["BTC", "ETH", "SOL", "XRP", "ADA", "LINK", "BNB"];
const MAX_DATA_STALENESS: Duration = Duration::from_secs(30);

/// Price entry stored in memory
#[derive(Debug, Clone)]
pub struct TickerPriceEntry {
    pub price_usd: f64,
    pub change_24h: f64,
    pub last_updated: Instant,
}

/// Payload received from Binance combined stream: `stream?streams=...`
#[derive(Debug, Deserialize)]
pub struct BinanceWsCombinedStreamPayload {
    #[serde(default)]
    pub stream: String,
    pub data: BinanceWsTickerData,
}

/// 24hr ticker data from Binance WebSocket
#[derive(Debug, Deserialize)]
pub struct BinanceWsTickerData {
    /// Symbol (e.g. "BTCUSDT")
    #[serde(rename = "s")]
    pub symbol: String,

    /// Last price (e.g. "95234.12")
    #[serde(rename = "c")]
    pub last_price: String,

    /// Price change percent (e.g. "2.45")
    #[serde(rename = "P")]
    pub price_change_percent: String,
}

/// Binance WebSocket Client managing real-time price feeds
pub struct BinanceWsClient {
    /// In-memory store: Symbol ("BTC", "ETH", etc.) -> `TickerPriceEntry`
    prices: Arc<DashMap<String, TickerPriceEntry>>,

    /// Connection status
    connected: Arc<AtomicBool>,

    /// Timestamp of last received message (unix seconds)
    last_message_timestamp: Arc<AtomicU64>,

    /// Flag indicating whether we are using Binance US fallback
    using_us_fallback: Arc<AtomicBool>,
}

impl BinanceWsClient {
    /// Create and spawn a new `BinanceWsClient`
    #[must_use]
    pub fn new() -> Arc<Self> {
        let client = Arc::new(Self {
            prices: Arc::new(DashMap::new()),
            connected: Arc::new(AtomicBool::new(false)),
            last_message_timestamp: Arc::new(AtomicU64::new(0)),
            using_us_fallback: Arc::new(AtomicBool::new(false)),
        });

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let client_clone = Arc::clone(&client);
            handle.spawn(async move {
                client_clone.run_connection_loop().await;
            });
        }

        client
    }

    /// Retrieve all 7 crypto prices if all are present and fresh (updated within 30s)
    /// Returns `None` if any coin is missing or stale, signaling fallback to HTTP REST API.
    #[must_use]
    pub fn get_multi_crypto_prices(&self) -> Option<HashMap<String, CryptoPrice>> {
        let mut result = HashMap::with_capacity(TRACKED_COINS.len());

        for &coin in &TRACKED_COINS {
            if let Some(entry) = self.prices.get(coin) {
                if entry.last_updated.elapsed() > MAX_DATA_STALENESS {
                    warn!(
                        coin = %coin,
                        age_secs = entry.last_updated.elapsed().as_secs(),
                        "Binance WebSocket price is stale"
                    );
                    return None;
                }

                if entry.price_usd <= 0.0 {
                    warn!(coin = %coin, price = entry.price_usd, "Invalid price in WebSocket cache");
                    return None;
                }

                result.insert(
                    coin.to_string(),
                    CryptoPrice::new(entry.price_usd, entry.change_24h),
                );
            } else {
                debug!(coin = %coin, "Missing coin in Binance WebSocket cache");
                return None;
            }
        }

        if result.len() == TRACKED_COINS.len() {
            Some(result)
        } else {
            None
        }
    }

    /// Manually update or seed a price entry (e.g. from HTTP fallback response)
    pub fn update_price(&self, coin: &str, price_usd: f64, change_24h: f64) {
        self.prices.insert(
            coin.to_string(),
            TickerPriceEntry {
                price_usd,
                change_24h,
                last_updated: Instant::now(),
            },
        );
    }

    /// Seed multiple prices from HTTP fallback
    pub fn seed_prices(&self, prices: &HashMap<String, CryptoPrice>) {
        for (coin, price) in prices {
            self.update_price(coin, price.price_usd, price.change_24h);
        }
    }

    /// Check if WebSocket client is healthy and actively receiving data
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        let is_connected = self.connected.load(Ordering::Relaxed);
        let last_msg = self.last_message_timestamp.load(Ordering::Relaxed);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        is_connected && (now.saturating_sub(last_msg) < 60)
    }

    /// Convert Binance symbol to tracked coin name
    fn symbol_to_coin(symbol: &str) -> Option<&'static str> {
        match symbol {
            "BTCUSDT" => Some("BTC"),
            "ETHUSDT" => Some("ETH"),
            "SOLUSDT" => Some("SOL"),
            "XRPUSDT" => Some("XRP"),
            "ADAUSDT" => Some("ADA"),
            "LINKUSDT" => Some("LINK"),
            "BNBUSDT" => Some("BNB"),
            _ => None,
        }
    }

    /// Process incoming text message from Binance WebSocket
    fn process_message(&self, text: &str) {
        // Try parsing combined stream payload
        if let Ok(payload) = serde_json::from_str::<BinanceWsCombinedStreamPayload>(text) {
            self.handle_ticker_data(&payload.data);
            return;
        }

        // Try parsing single ticker payload directly as fallback
        if let Ok(data) = serde_json::from_str::<BinanceWsTickerData>(text) {
            self.handle_ticker_data(&data);
        }
    }

    /// Handle parsed ticker data
    fn handle_ticker_data(&self, data: &BinanceWsTickerData) {
        if let Some(coin) = Self::symbol_to_coin(&data.symbol) {
            let price_usd: f64 = data.last_price.parse().unwrap_or(0.0);
            let change_24h: f64 = data.price_change_percent.parse().unwrap_or(0.0);

            if price_usd > 0.0 {
                self.update_price(coin, price_usd, change_24h);

                let now_secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());

                self.last_message_timestamp
                    .store(now_secs, Ordering::Relaxed);
            }
        }
    }

    /// Connection loop with automatic reconnect and endpoint switching
    async fn run_connection_loop(&self) {
        let mut retry_count: u32 = 0;
        let mut use_us = self.using_us_fallback.load(Ordering::Relaxed);

        loop {
            let url = if use_us {
                BINANCE_US_WS_STREAM_URL
            } else {
                BINANCE_WS_STREAM_URL
            };

            info!(url = %url, "Connecting to Binance WebSocket stream...");

            match connect_async(url).await {
                Ok((ws_stream, response)) => {
                    info!(
                        status = %response.status(),
                        "✅ Connected to Binance WebSocket stream"
                    );

                    self.connected.store(true, Ordering::Relaxed);
                    self.using_us_fallback.store(use_us, Ordering::Relaxed);
                    retry_count = 0;

                    let (_, mut read) = ws_stream.split();

                    while let Some(msg_result) = read.next().await {
                        match msg_result {
                            Ok(Message::Text(text)) => {
                                self.process_message(&text);
                            }
                            Ok(Message::Ping(_)) => {
                                debug!("Received Ping from Binance WebSocket");
                            }
                            Ok(Message::Pong(_)) => {
                                debug!("Received Pong from Binance WebSocket");
                            }
                            Ok(Message::Close(frame)) => {
                                warn!(frame = ?frame, "Binance WebSocket server closed connection");
                                break;
                            }
                            Err(e) => {
                                error!(error = %e, "Error reading from Binance WebSocket");
                                break;
                            }
                            _ => {}
                        }
                    }

                    self.connected.store(false, Ordering::Relaxed);
                    warn!("Binance WebSocket connection disconnected");
                }
                Err(e) => {
                    self.connected.store(false, Ordering::Relaxed);
                    error!(error = %e, url = %url, "Failed to connect to Binance WebSocket");

                    // Check if we should switch to Binance US
                    let error_str = e.to_string();
                    if (error_str.contains("451") || retry_count >= 2) && !use_us {
                        warn!("Switching to Binance US WebSocket fallback");
                        use_us = true;
                    }
                }
            }

            // Exponential backoff: 1s, 2s, 4s, 8s, max 15s
            retry_count = retry_count.saturating_add(1);
            let backoff_secs = 2_u64.pow(retry_count.min(4)).min(15);
            info!(
                retry_in_secs = backoff_secs,
                "Reconnecting to Binance WebSocket in {} seconds...", backoff_secs
            );
            sleep(Duration::from_secs(backoff_secs)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_to_coin() {
        assert_eq!(BinanceWsClient::symbol_to_coin("BTCUSDT"), Some("BTC"));
        assert_eq!(BinanceWsClient::symbol_to_coin("ETHUSDT"), Some("ETH"));
        assert_eq!(BinanceWsClient::symbol_to_coin("SOLUSDT"), Some("SOL"));
        assert_eq!(BinanceWsClient::symbol_to_coin("XRPUSDT"), Some("XRP"));
        assert_eq!(BinanceWsClient::symbol_to_coin("ADAUSDT"), Some("ADA"));
        assert_eq!(BinanceWsClient::symbol_to_coin("LINKUSDT"), Some("LINK"));
        assert_eq!(BinanceWsClient::symbol_to_coin("BNBUSDT"), Some("BNB"));
        assert_eq!(BinanceWsClient::symbol_to_coin("DOGEUSDT"), None);
    }

    #[test]
    fn test_process_combined_stream_message() {
        let client = BinanceWsClient::new();

        let json_msg = r#"{
            "stream": "btcusdt@ticker",
            "data": {
                "e": "24hrTicker",
                "E": 123456789,
                "s": "BTCUSDT",
                "p": "500.0",
                "P": "2.5",
                "c": "96500.50"
            }
        }"#;

        client.process_message(json_msg);

        let entry = client.prices.get("BTC");
        assert!(entry.is_some());
        if let Some(btc) = entry {
            assert!((btc.price_usd - 96500.50).abs() < f64::EPSILON);
            assert!((btc.change_24h - 2.5).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn test_get_multi_crypto_prices_completeness() {
        let client = BinanceWsClient::new();

        // Initially empty -> returns None
        assert!(client.get_multi_crypto_prices().is_none());

        // Partial coins -> returns None
        client.update_price("BTC", 95000.0, 1.2);
        client.update_price("ETH", 3500.0, 2.0);
        assert!(client.get_multi_crypto_prices().is_none());

        // All 7 coins present and fresh
        client.update_price("SOL", 200.0, 3.5);
        client.update_price("XRP", 2.5, 0.8);
        client.update_price("ADA", 0.9, -1.0);
        client.update_price("LINK", 22.0, 4.0);
        client.update_price("BNB", 650.0, 0.5);

        let prices = client.get_multi_crypto_prices();
        assert!(prices.is_some());
        if let Some(p) = prices {
            assert_eq!(p.len(), 7);
            if let Some(btc) = p.get("BTC") {
                assert!((btc.price_usd - 95000.0).abs() < f64::EPSILON);
                assert!((btc.change_24h - 1.2).abs() < f64::EPSILON);
            } else {
                panic!("BTC price missing");
            }
        }
    }

    #[test]
    fn test_seed_prices() {
        let client = BinanceWsClient::new();
        let mut map = HashMap::new();
        map.insert("BTC".to_string(), CryptoPrice::new(90000.0, 3.0));
        map.insert("ETH".to_string(), CryptoPrice::new(3000.0, 1.5));
        map.insert("SOL".to_string(), CryptoPrice::new(180.0, 2.0));
        map.insert("XRP".to_string(), CryptoPrice::new(2.0, 0.5));
        map.insert("ADA".to_string(), CryptoPrice::new(0.8, 1.0));
        map.insert("LINK".to_string(), CryptoPrice::new(20.0, -0.5));
        map.insert("BNB".to_string(), CryptoPrice::new(600.0, 1.2));

        client.seed_prices(&map);
        let prices = client.get_multi_crypto_prices();
        assert!(prices.is_some());
    }
}
