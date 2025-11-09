//! Market Data Streamer Component
//! 
//! This component streams real-time market data from Layer 2 External APIs
//! to connected WebSocket clients, following Service Islands Architecture.

use anyhow::Result;
use serde_json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::interval;

use crate::service_islands::layer2_external_services::external_apis_island::ExternalApisIsland;
use crate::service_islands::ServiceIslands;

/// Market Data Streamer
/// 
/// Streams real-time market data from External APIs to WebSocket clients.
/// This component bridges Layer 2 (External Services) with Layer 3 (Communication).
pub struct MarketDataStreamer {
    /// Reference to Layer 2 External APIs
    external_apis: Option<Arc<ExternalApisIsland>>,
    /// Reference to Service Islands for Layer 5 access
    service_islands: Option<Arc<ServiceIslands>>,
    /// Stream interval for updates
    update_interval: Duration,
    /// Active streaming flag
    is_streaming: std::sync::atomic::AtomicBool,
}

impl MarketDataStreamer {
    /// Create new Market Data Streamer without External APIs dependency
    pub fn new() -> Self {
        Self {
            external_apis: None,
            service_islands: None,
            update_interval: Duration::from_secs(5), // 5 seconds - reduced load for client and server
            is_streaming: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Set Service Islands reference for Layer 5 access (compatibility method)
    pub fn with_service_islands(mut self, service_islands: Arc<ServiceIslands>) -> Self {
        self.service_islands = Some(service_islands);
        self
    }
    
    /// Health check for market data streamer
    /// 
    /// Begins periodic streaming of market data using Layer 5 → Layer 3 → Layer 2 flow
    /// to match the same data source as HTTP API and WebSocket initial messages.
    pub async fn start_streaming(&self, broadcast_tx: broadcast::Sender<String>) -> Result<()> {
        if let Some(service_islands) = &self.service_islands {
            println!("🌊 Starting market data streaming using Layer 5 → Layer 3 → Layer 2 flow...");
            
            self.is_streaming.store(true, std::sync::atomic::Ordering::Relaxed);
            
            // Clone Arc pointers to move into spawned task (tokio::spawn requires 'static lifetime)
            // These are cheap operations (~5-10ns each) - only increment reference counters
            let service_islands_clone = Arc::clone(&service_islands);
            let broadcast_tx_clone = broadcast_tx.clone(); // broadcast::Sender internally uses Arc
            let update_interval = self.update_interval;
            
            // Spawn background task for streaming (runs independently for application lifetime)
            tokio::spawn(async move {
                let mut interval_timer = interval(update_interval);
                let mut consecutive_failures = 0;
                let max_consecutive_failures = 5;
                
                loop {
                    interval_timer.tick().await;
                    
                    // Check if broadcast channel is still active
                    if broadcast_tx_clone.receiver_count() == 0 {
                        println!("📡 No WebSocket receivers - continuing to stream for future connections");
                    }
                    
                    // 🔧 FIX: Use same Layer 5 function as HTTP API and WebSocket initial message
                    // This ensures all three messages use identical Layer 2 access pattern
                    match service_islands_clone.websocket_service.fetch_market_data(true).await {
                        Ok(dashboard_data) => {
                            // Reset consecutive failures on success
                            consecutive_failures = 0;
                            
                            // 🔍 DEBUG: Log detailed dashboard data values
                            println!("🔍 [DEBUG] Dashboard data fetched successfully:");
                            if let Some(market_cap) = dashboard_data.get("market_cap_usd") {
                                println!("  💰 Market Cap: ${:?}", market_cap);
                            }
                            if let Some(volume) = dashboard_data.get("volume_24h_usd") {
                                println!("  📊 24h Volume: ${:?}", volume);
                            }
                            if let Some(btc_price) = dashboard_data.get("btc_price_usd") {
                                println!("  ₿ BTC Price: ${:?}", btc_price);
                            }
                            if let Some(btc_change) = dashboard_data.get("btc_change_24h") {
                                println!("  📈 BTC 24h Change: {:?}%", btc_change);
                            }
                            if let Some(fng) = dashboard_data.get("fng_value") {
                                println!("  😨 Fear & Greed Index: {:?}", fng);
                            }
                            if let Some(btc_rsi_14) = dashboard_data.get("btc_rsi_14") {
                                println!("  📈 RSI 14: {:?}", btc_rsi_14);
                            }
                            if let Some(btc_dom) = dashboard_data.get("btc_market_cap_percentage") {
                                println!("  ₿ BTC Dominance: {:?}%", btc_dom);
                            }
                            if let Some(eth_dom) = dashboard_data.get("eth_market_cap_percentage") {
                                println!("  Ξ ETH Dominance: {:?}%", eth_dom);
                            }
                            if let Some(eth_price) = dashboard_data.get("eth_price_usd") {
                                println!("  Ξ ETH Price: ${:?}", eth_price);
                            }
                            if let Some(eth_change) = dashboard_data.get("eth_change_24h") {
                                println!("  📈 ETH 24h Change: {:?}%", eth_change);
                            }
                            if let Some(sol_price) = dashboard_data.get("sol_price_usd") {
                                println!("  ◎ SOL Price: ${:?}", sol_price);
                            }
                            if let Some(sol_change) = dashboard_data.get("sol_change_24h") {
                                println!("  📈 SOL 24h Change: {:?}%", sol_change);
                            }
                            if let Some(xrp_price) = dashboard_data.get("xrp_price_usd") {
                                println!("  💧 XRP Price: ${:?}", xrp_price);
                            }
                            if let Some(xrp_change) = dashboard_data.get("xrp_change_24h") {
                                println!("  📈 XRP 24h Change: {:?}%", xrp_change);
                            }
                            if let Some(ada_price) = dashboard_data.get("ada_price_usd") {
                                println!("  🃏 ADA Price: ${:?}", ada_price);
                            }
                            if let Some(ada_change) = dashboard_data.get("ada_change_24h") {
                                println!("  📈 ADA 24h Change: {:?}%", ada_change);
                            }
                            if let Some(link_price) = dashboard_data.get("link_price_usd") {
                                println!("  🔗 LINK Price: ${:?}", link_price);
                            }
                            if let Some(link_change) = dashboard_data.get("link_change_24h") {
                                println!("  📈 LINK 24h Change: {:?}%", link_change);
                            }
                            if let Some(bnb_price) = dashboard_data.get("bnb_price_usd") {
                                println!("  🪙 BNB Price: ${:?}", bnb_price);
                            }
                            if let Some(bnb_change) = dashboard_data.get("bnb_change_24h") {
                                println!("  📈 BNB 24h Change: {:?}%", bnb_change);
                            }

                            if let Some(us_indices) = dashboard_data.get("us_stock_indices") {
                                println!("  📈 US Stock Indices: {:?}", us_indices);
                            }
                            let ws_message = serde_json::json!({
                                "type": "dashboard_update",
                                "data": dashboard_data,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                "source": "external_apis"
                            });
                            
                            // Broadcast to all WebSocket clients
                            let message_str = ws_message.to_string();
                            match broadcast_tx_clone.send(message_str) {
                                Ok(receiver_count) => {
                                    println!("📊 Dashboard data broadcasted to {} WebSocket clients", receiver_count);
                                }
                                Err(e) => {
                                    // This is expected when no clients are connected
                                    println!("� Broadcast ready - waiting for WebSocket connections ({})", e);
                                }
                            }
                        }
                        Err(e) => {
                            consecutive_failures += 1;
                            println!("⚠️ Failed to fetch dashboard data for streaming (attempt {}/{}): {}", 
                                consecutive_failures, max_consecutive_failures, e);
                            
                            // Only broadcast error after multiple failures to avoid spam
                            if consecutive_failures >= 3 {
                                let error_message = serde_json::json!({
                                    "type": "error",
                                    "message": "Temporary issue with real-time market data",
                                    "error": e.to_string(),
                                    "consecutive_failures": consecutive_failures,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                
                                let _ = broadcast_tx_clone.send(error_message.to_string());
                            }
                            
                            // If too many failures, increase interval temporarily
                            if consecutive_failures >= max_consecutive_failures {
                                println!("⚠️ Too many consecutive failures - taking a break");
                                tokio::time::sleep(Duration::from_secs(60)).await; // 1 minute break
                                consecutive_failures = 0; // Reset counter after break
                            }
                        }
                    }
                }
            });
            
            println!("✅ Market data streaming started successfully");
            Ok(())
        } else {
            println!("⚠️ Service Islands not configured - market data streaming disabled");
            Ok(())
        }
    }
    
    /// Health check for market data streamer
    /// 
    /// Improved health check that's more tolerant of temporary API issues.
    pub async fn health_check(&self) -> bool {
        if let Some(external_apis) = &self.external_apis {
            match external_apis.health_check().await {
                Ok(_) => {
                    println!("  ✅ Market Data Streamer - External APIs healthy");
                    true
                }
                Err(e) => {
                    // Check if this is just a rate limit or circuit breaker issue
                    let error_msg = e.to_string();
                    if error_msg.contains("429") || error_msg.contains("Circuit breaker") || error_msg.contains("rate limit") {
                        println!("  ⚠️ Market Data Streamer - External APIs rate limited (still functional)");
                        true // Consider rate limiting as "healthy" since it's temporary
                    } else {
                        println!("  ❌ Market Data Streamer - External APIs unhealthy: {}", e);
                        false
                    }
                }
            }
        } else {
            println!("  ⚠️ Market Data Streamer - External APIs not configured (test mode)");
            true // Not an error, just not configured
        }
    }
}
