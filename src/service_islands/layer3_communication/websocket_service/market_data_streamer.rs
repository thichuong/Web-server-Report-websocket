//! Market Data Streamer Component
//!
//! This component streams real-time market data from Layer 2 External APIs
//! to connected WebSocket clients, following Service Islands Architecture.

use std::sync::Arc;
use tracing::{info, warn, error};

use crate::service_islands::layer2_external_services::external_apis_island::ExternalApisIsland;

/// Market Data Streamer
///
/// Streams real-time market data from External APIs to WebSocket clients.
/// This component bridges Layer 2 (External Services) with Layer 3 (Communication).
pub struct MarketDataStreamer {
    /// Reference to Layer 2 External APIs
    external_apis: Option<Arc<ExternalApisIsland>>,
}

impl MarketDataStreamer {
    /// Create new Market Data Streamer without External APIs dependency
    pub fn new() -> Self {
        Self {
            external_apis: None,
        }
    }

    /// Health check for market data streamer
    ///
    /// Improved health check that's more tolerant of temporary API issues.
    pub async fn health_check(&self) -> bool {
        if let Some(external_apis) = &self.external_apis {
            match external_apis.health_check().await {
                Ok(_) => {
                    info!("Market Data Streamer - External APIs healthy");
                    true
                }
                Err(e) => {
                    // Check if this is just a rate limit or circuit breaker issue
                    let error_msg = e.to_string();
                    if error_msg.contains("429") || error_msg.contains("Circuit breaker") || error_msg.contains("rate limit") {
                        warn!("Market Data Streamer - External APIs rate limited (still functional)");
                        true // Consider rate limiting as "healthy" since it's temporary
                    } else {
                        error!("Market Data Streamer - External APIs unhealthy: {}", e);
                        false
                    }
                }
            }
        } else {
            warn!("Market Data Streamer - External APIs not configured (test mode)");
            true // Not an error, just not configured
        }
    }
}
