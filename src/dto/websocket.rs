//! WebSocket DTO Layer
//!
//! This module defines the strict API contract for WebSocket communication
//! between the client (frontend) and the server using adjacently-tagged enums
//! for easy parsing.

use chrono::Utc;
use serde::{Deserialize, Serialize};

// ============================================================================
// Crypto Price Data (Internal API format)
// ============================================================================

/// Cryptocurrency price data from external APIs
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CryptoPrice {
    /// Price in USD
    pub price_usd: f64,

    /// 24-hour price change percentage
    pub change_24h: f64,
}

impl CryptoPrice {
    /// Create a new `CryptoPrice`
    #[must_use]
    pub fn new(price_usd: f64, change_24h: f64) -> Self {
        Self {
            price_usd,
            change_24h,
        }
    }

    /// Default/zero values for fallback
    #[must_use]
    pub fn zero() -> Self {
        Self {
            price_usd: 0.0,
            change_24h: 0.0,
        }
    }
}

impl Default for CryptoPrice {
    fn default() -> Self {
        Self::zero()
    }
}



// ============================================================================
// Server Messages (Server → Client)
// ============================================================================

/// Messages sent FROM the server TO the client.
///
/// Uses adjacently-tagged enum format for easy frontend parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum ServerMessage {
    /// Welcome message sent immediately upon connection
    Welcome(WelcomePayload),

    /// Individual market data update for a specific symbol
    MarketUpdate(MarketUpdatePayload),

    /// Full dashboard update with all market data (current implementation)
    DashboardUpdate(Box<DashboardUpdatePayload>),

    /// System health status update
    SystemHealth(SystemHealthPayload),

    /// Error message
    Error(ErrorPayload),

    /// Acknowledgment of subscription/unsubscription
    Ack(AckPayload),
}

impl ServerMessage {
    /// Serialize to JSON string for sending via WebSocket
    ///
    /// # Errors
    /// Returns `serde_json::Error` if serialization fails due to malformed data structure
    #[allow(dead_code)]
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

// ============================================================================
// Server Message Payloads
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct WelcomePayload {
    /// Unique connection identifier
    pub connection_id: String,

    /// Server version information
    pub server_version: String,

    /// Connection timestamp (RFC3339 format)
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct MarketUpdatePayload {
    /// Symbol/ticker (e.g., "BTC", "ETH", "SOL")
    pub symbol: String,

    /// Current price in USD
    pub price: f64,

    /// 24-hour price change percentage
    pub change_24h: f64,

    /// 24-hour trading volume (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,

    /// Unix timestamp in milliseconds
    pub timestamp: i64,
}

/// Strongly-typed dashboard data structure
///
/// Matches the exact structure from Redis stream and `dashboard_aggregator`.
/// Accepts `snake_case` from Redis (via aliases) and outputs camelCase to frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DashboardData {
    // BTC data
    #[serde(alias = "btc_price_usd")]
    pub btc_price_usd: f64,
    #[serde(alias = "btc_change_24h")]
    pub btc_change_24h: f64,
    #[serde(alias = "btc_market_cap_percentage")]
    pub btc_market_cap_percentage: f64,
    #[serde(alias = "btc_rsi_14")]
    pub btc_rsi_14: f64,

    // ETH data
    #[serde(alias = "eth_price_usd")]
    pub eth_price_usd: f64,
    #[serde(alias = "eth_change_24h")]
    pub eth_change_24h: f64,
    #[serde(alias = "eth_market_cap_percentage")]
    pub eth_market_cap_percentage: f64,

    // SOL data
    #[serde(alias = "sol_price_usd")]
    pub sol_price_usd: f64,
    #[serde(alias = "sol_change_24h")]
    pub sol_change_24h: f64,

    // XRP data
    #[serde(alias = "xrp_price_usd")]
    pub xrp_price_usd: f64,
    #[serde(alias = "xrp_change_24h")]
    pub xrp_change_24h: f64,

    // ADA data
    #[serde(alias = "ada_price_usd")]
    pub ada_price_usd: f64,
    #[serde(alias = "ada_change_24h")]
    pub ada_change_24h: f64,

    // LINK data
    #[serde(alias = "link_price_usd")]
    pub link_price_usd: f64,
    #[serde(alias = "link_change_24h")]
    pub link_change_24h: f64,

    // BNB data
    #[serde(alias = "bnb_price_usd")]
    pub bnb_price_usd: f64,
    #[serde(alias = "bnb_change_24h")]
    pub bnb_change_24h: f64,

    // Global market data
    #[serde(alias = "market_cap_usd")]
    pub market_cap_usd: f64,
    #[serde(alias = "volume_24h_usd")]
    pub volume_24h_usd: f64,
    #[serde(alias = "market_cap_change_percentage_24h_usd")]
    pub market_cap_change_percentage_24h_usd: f64,

    // Indicators
    #[serde(alias = "fng_value")]
    pub fng_value: u32,

    // Metadata
    #[serde(alias = "fetch_duration_ms")]
    pub fetch_duration_ms: u64,
    #[serde(alias = "partial_failure")]
    pub partial_failure: bool,
    #[serde(alias = "last_updated")]
    pub last_updated: String,
    #[serde(alias = "timestamp")]
    pub timestamp: String,
}

impl DashboardData {
    /// Serialize to JSON string
    ///
    /// # Errors
    /// Returns `serde_json::Error` if serialization fails due to malformed data structure
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct DashboardUpdatePayload {
    /// Complete dashboard data (strongly-typed structure)
    /// Aligns with current `dashboard_summary_v2` implementation
    pub data: DashboardData,

    /// Timestamp (RFC3339 format)
    pub timestamp: String,

    /// Data source identifier
    pub source: String,
}

impl DashboardUpdatePayload {
    /// Create a new dashboard update from `DashboardData`
    #[must_use]
    #[allow(dead_code)]
    pub fn new(data: DashboardData, source: &str) -> Self {
        Self {
            data,
            timestamp: Utc::now().to_rfc3339(),
            source: source.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct SystemHealthPayload {
    /// Overall system status
    pub status: HealthStatus,

    /// Health status per service island layer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_health: Option<LayerHealth>,

    /// Unix timestamp
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LayerHealth {
    /// Layer 1: Infrastructure (cache, coordination)
    pub infrastructure: bool,

    /// Layer 2: External APIs
    pub external_apis: bool,

    /// Layer 3: WebSocket communication
    pub websocket: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ErrorPayload {
    /// Error code (use `ERROR_CODE`_* constants)
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Unix timestamp
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct AckPayload {
    /// Action that was acknowledged ("subscribe" or "unsubscribe")
    pub action: String,

    /// Topics that were successfully processed
    pub topics: Vec<String>,

    /// Unix timestamp
    pub timestamp: i64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    // Mock client message for testing since it's not defined in this file
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type")]
    enum MockClientMessage {
        Subscribe(SubscribePayload),
        Heartbeat,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SubscribePayload {
        topics: Vec<String>,
    }

    #[test]
    fn test_client_message_subscribe_serialization() {
        let msg = MockClientMessage::Subscribe(SubscribePayload {
            topics: vec!["BTC".to_string(), "ETH".to_string()],
        });

        let json = serde_json::to_string(&msg).expect("Serialization should succeed");
        assert!(json.contains(r#""type":"Subscribe"#));
        assert!(json.contains(r#""topics":["BTC","ETH"]"#));
    }

    #[test]
    fn test_client_message_heartbeat() {
        let msg = MockClientMessage::Heartbeat;
        let json = serde_json::to_string(&msg).expect("Serialization should succeed");
        assert_eq!(json, r#"{"type":"Heartbeat"}"#);
    }

    #[test]
    fn test_server_message_welcome() {
        let payload = WelcomePayload {
            connection_id: "conn-123".to_string(),
            server_version: "1.0.0".to_string(),
            timestamp: "2023-01-01T00:00:00Z".to_string(),
        };
        let msg = ServerMessage::Welcome(payload);
        let json = msg.to_json_string().expect("Serialization should succeed");

        assert!(json.contains(r#""type":"Welcome"#));
        assert!(json.contains("conn-123"));
        assert!(json.contains("1.0.0"));
    }

    #[test]
    fn test_market_update_camel_case() {
        let msg = ServerMessage::MarketUpdate(MarketUpdatePayload {
            symbol: "BTC".to_string(),
            price: 50000.0,
            change_24h: 5.2,
            volume: Some(1_000_000.0),
            timestamp: 1_234_567_890,
        });

        let json = msg.to_json_string().expect("Serialization should succeed");
        assert!(json.contains("change24h")); // camelCase field name
        assert!(json.contains("50000"));
    }

    #[test]
    fn test_dashboard_data_serialization() {
        let dashboard_data = DashboardData {
            btc_price_usd: 50000.0,
            btc_change_24h: 2.5,
            btc_market_cap_percentage: 50.0,
            btc_rsi_14: 60.0,
            eth_price_usd: 3000.0,
            eth_change_24h: 1.5,
            eth_market_cap_percentage: 20.0,
            sol_price_usd: 100.0,
            sol_change_24h: 5.0,
            xrp_price_usd: 0.5,
            xrp_change_24h: 0.1,
            ada_price_usd: 0.4,
            ada_change_24h: 0.2,
            link_price_usd: 15.0,
            link_change_24h: 1.0,
            bnb_price_usd: 300.0,
            bnb_change_24h: 0.5,
            market_cap_usd: 2_000_000_000.0,
            volume_24h_usd: 1_000_000_000.0,
            market_cap_change_percentage_24h_usd: 1.2,
            fng_value: 50,
            fetch_duration_ms: 100,
            partial_failure: false,
            last_updated: "2023-01-01T00:00:00Z".to_string(),
            timestamp: "2023-01-01T00:00:00Z".to_string(),
        };

        let json = dashboard_data
            .to_json_string()
            .expect("Serialization should succeed");

        assert!(json.contains("btc_price_usd"));
        assert!(json.contains("50000"));
    }

    #[test]
    fn test_dashboard_update_payload_serialization() {
        let dashboard_data = DashboardData {
            btc_price_usd: 50000.0,
            btc_change_24h: 2.5,
            btc_market_cap_percentage: 50.0,
            btc_rsi_14: 60.0,
            eth_price_usd: 3000.0,
            eth_change_24h: 1.5,
            eth_market_cap_percentage: 20.0,
            sol_price_usd: 100.0,
            sol_change_24h: 5.0,
            xrp_price_usd: 0.5,
            xrp_change_24h: 0.1,
            ada_price_usd: 0.4,
            ada_change_24h: 0.2,
            link_price_usd: 15.0,
            link_change_24h: 1.0,
            bnb_price_usd: 300.0,
            bnb_change_24h: 0.5,
            market_cap_usd: 2_000_000_000.0,
            volume_24h_usd: 1_000_000_000.0,
            market_cap_change_percentage_24h_usd: 1.2,
            fng_value: 50,
            fetch_duration_ms: 100,
            partial_failure: false,
            last_updated: "2023-01-01T00:00:00Z".to_string(),
            timestamp: "2023-01-01T00:00:00Z".to_string(),
        };

        let payload = DashboardUpdatePayload::new(dashboard_data, "test_source");
        let msg = ServerMessage::DashboardUpdate(Box::new(payload));
        let json = msg.to_json_string().expect("Serialization should succeed");

        assert!(json.contains("DashboardUpdate"));
        assert!(json.contains("btc_price_usd"));
        assert!(json.contains("test_source"));
    }
}
