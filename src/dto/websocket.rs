//! WebSocket DTO Layer
//!
//! This module defines the strict API contract for WebSocket communication
//! between the client (frontend) and the server using adjacently-tagged enums
//! for easy parsing.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// Error Code Constants
// ============================================================================

pub const ERROR_CODE_INVALID_MESSAGE: &str = "INVALID_MESSAGE";
pub const ERROR_CODE_INVALID_TOPIC: &str = "INVALID_TOPIC";
pub const ERROR_CODE_SUBSCRIPTION_FAILED: &str = "SUBSCRIPTION_FAILED";
pub const ERROR_CODE_UNSUBSCRIBE_FAILED: &str = "UNSUBSCRIBE_FAILED";
pub const ERROR_CODE_INTERNAL_ERROR: &str = "INTERNAL_ERROR";
pub const ERROR_CODE_RATE_LIMITED: &str = "RATE_LIMITED";

// ============================================================================
// Client Messages (Client → Server)
// ============================================================================

/// Messages sent FROM the client TO the server.
///
/// Uses adjacently-tagged enum format for easy frontend parsing:
/// ```json
/// {
///   "type": "Subscribe",
///   "payload": { "topics": ["BTC", "ETH"] }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    /// Subscribe to specific topics or symbols
    Subscribe(SubscribePayload),

    /// Unsubscribe from topics
    Unsubscribe(UnsubscribePayload),

    /// Heartbeat/ping to keep connection alive
    Heartbeat,
}

impl ClientMessage {
    /// Parse a ClientMessage from a JSON string
    ///
    /// # Example
    /// ```
    /// let msg = ClientMessage::from_json_str(r#"{"type":"Heartbeat"}"#)?;
    /// ```
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

// ============================================================================
// Server Messages (Server → Client)
// ============================================================================

/// Messages sent FROM the server TO the client.
///
/// Uses adjacently-tagged enum format for easy frontend parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    /// Welcome message sent immediately upon connection
    Welcome(WelcomePayload),

    /// Individual market data update for a specific symbol
    MarketUpdate(MarketUpdatePayload),

    /// Full dashboard update with all market data (current implementation)
    DashboardUpdate(DashboardUpdatePayload),

    /// System health status update
    SystemHealth(SystemHealthPayload),

    /// Error message
    Error(ErrorPayload),

    /// Acknowledgment of subscription/unsubscription
    Ack(AckPayload),
}

impl ServerMessage {
    /// Create a new error message
    ///
    /// # Example
    /// ```
    /// let error = ServerMessage::new_error(
    ///     ERROR_CODE_INVALID_TOPIC,
    ///     "Topic 'INVALID' does not exist"
    /// );
    /// ```
    pub fn new_error(code: &str, message: &str) -> Self {
        ServerMessage::Error(ErrorPayload {
            code: code.to_string(),
            message: message.to_string(),
            timestamp: Utc::now().timestamp(),
        })
    }

    /// Create a welcome message
    pub fn new_welcome(connection_id: String, server_version: &str) -> Self {
        ServerMessage::Welcome(WelcomePayload {
            connection_id,
            server_version: server_version.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        })
    }

    /// Create an acknowledgment message
    pub fn new_ack(action: &str, topics: Vec<String>) -> Self {
        ServerMessage::Ack(AckPayload {
            action: action.to_string(),
            topics,
            timestamp: Utc::now().timestamp(),
        })
    }

    /// Serialize to JSON string for sending via WebSocket
    ///
    /// # Example
    /// ```
    /// let msg = ServerMessage::new_error("ERR001", "Something went wrong");
    /// let json_str = msg.to_json_string()?;
    /// socket.send(Message::Text(json_str)).await?;
    /// ```
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

// ============================================================================
// Client Message Payloads
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribePayload {
    /// List of topics/symbols to subscribe to
    /// Examples: ["BTC", "ETH", "MarketStats", "SystemHealth"]
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribePayload {
    /// List of topics/symbols to unsubscribe from
    pub topics: Vec<String>,
}

// ============================================================================
// Server Message Payloads
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
/// Matches the exact structure from Redis stream and dashboard_aggregator.
/// Accepts snake_case from Redis (via aliases) and outputs camelCase to frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    // US Stock Indices (nested object)
    #[serde(alias = "us_stock_indices")]
    pub us_stock_indices: Value,

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
    /// Deserialize from JSON string (e.g., from Redis stream)
    ///
    /// # Example
    /// ```
    /// let json = r#"{"btc_price_usd": 96000.0, "fng_value": 10, ...}"#;
    /// let data = DashboardData::from_json_str(json)?;
    /// ```
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Serialize to JSON string
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardUpdatePayload {
    /// Complete dashboard data (strongly-typed structure)
    /// Aligns with current dashboard_summary_v2 implementation
    pub data: DashboardData,

    /// Timestamp (RFC3339 format)
    pub timestamp: String,

    /// Data source identifier
    pub source: String,
}

impl DashboardUpdatePayload {
    /// Create a new dashboard update from DashboardData
    pub fn new(data: DashboardData, source: &str) -> Self {
        Self {
            data,
            timestamp: Utc::now().to_rfc3339(),
            source: source.to_string(),
        }
    }

    /// Create from JSON string (e.g., from Redis stream)
    ///
    /// # Example
    /// ```
    /// let redis_value = r#"{"btc_price_usd": 96000.0, "fng_value": 10, ...}"#;
    /// let payload = DashboardUpdatePayload::from_json_str(redis_value, "external_apis")?;
    /// ```
    pub fn from_json_str(json: &str, source: &str) -> Result<Self, serde_json::Error> {
        let data = DashboardData::from_json_str(json)?;
        Ok(Self::new(data, source))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub struct ErrorPayload {
    /// Error code (use ERROR_CODE_* constants)
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Unix timestamp
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
mod tests {
    use super::*;

    #[test]
    fn test_client_message_subscribe_serialization() {
        let msg = ClientMessage::Subscribe(SubscribePayload {
            topics: vec!["BTC".to_string(), "ETH".to_string()],
        });

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Subscribe"#));
        assert!(json.contains(r#""topics":["BTC","ETH"]"#));
    }

    #[test]
    fn test_client_message_heartbeat() {
        let msg = ClientMessage::Heartbeat;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"Heartbeat"}"#);
    }

    #[test]
    fn test_client_message_from_json() {
        let json = r#"{"type":"Subscribe","payload":{"topics":["BTC"]}}"#;
        let msg = ClientMessage::from_json_str(json).unwrap();

        match msg {
            ClientMessage::Subscribe(payload) => {
                assert_eq!(payload.topics, vec!["BTC"]);
            }
            _ => panic!("Expected Subscribe variant"),
        }
    }

    #[test]
    fn test_server_message_error() {
        let msg = ServerMessage::new_error(ERROR_CODE_INVALID_TOPIC, "Invalid topic");
        let json = msg.to_json_string().unwrap();

        assert!(json.contains(r#""type":"Error"#));
        assert!(json.contains(ERROR_CODE_INVALID_TOPIC));
        assert!(json.contains("Invalid topic"));
    }

    #[test]
    fn test_server_message_welcome() {
        let msg = ServerMessage::new_welcome("conn-123".to_string(), "1.0.0");
        let json = msg.to_json_string().unwrap();

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
            volume: Some(1000000.0),
            timestamp: 1234567890,
        });

        let json = msg.to_json_string().unwrap();
        assert!(json.contains("change24h")); // camelCase field name
        assert!(json.contains("50000"));
    }

    #[test]
    fn test_dashboard_data_from_redis_json() {
        // This is the actual JSON structure from Redis stream
        let redis_json = r#"{
            "btc_price_usd": 96062.47,
            "btc_change_24h": 1.475,
            "btc_market_cap_percentage": 57.244131652924715,
            "btc_rsi_14": 33.44840837091841,
            "eth_price_usd": 3177.25,
            "eth_change_24h": 2.95,
            "eth_market_cap_percentage": 11.43216612211846,
            "sol_price_usd": 141.15,
            "sol_change_24h": 3.24,
            "xrp_price_usd": 2.2593,
            "xrp_change_24h": 0.071,
            "ada_price_usd": 0.5071,
            "ada_change_24h": 0.795,
            "link_price_usd": 14.2,
            "link_change_24h": 1.646,
            "bnb_price_usd": 935.51,
            "bnb_change_24h": 4.13,
            "market_cap_usd": 3334519158862.682,
            "volume_24h_usd": 208615359377.3596,
            "market_cap_change_percentage_24h_usd": 0.8706429089114247,
            "fng_value": 10,
            "us_stock_indices": {},
            "fetch_duration_ms": 114,
            "partial_failure": false,
            "last_updated": "2025-11-15T13:45:35.496238881+00:00",
            "timestamp": "2025-11-15T13:45:35.496253484+00:00"
        }"#;

        // Deserialize from Redis JSON (snake_case)
        let dashboard_data = DashboardData::from_json_str(redis_json).unwrap();

        // Verify key fields
        assert_eq!(dashboard_data.btc_price_usd, 96062.47);
        assert_eq!(dashboard_data.fng_value, 10);
        assert_eq!(dashboard_data.eth_price_usd, 3177.25);

        // Serialize back to JSON (should be camelCase for frontend)
        let json = dashboard_data.to_json_string().unwrap();
        assert!(json.contains("btcPriceUsd")); // camelCase
        assert!(json.contains("96062.47"));
    }

    #[test]
    fn test_dashboard_update_payload_from_redis() {
        let redis_json = r#"{
            "btc_price_usd": 96062.47,
            "btc_change_24h": 1.475,
            "btc_market_cap_percentage": 57.244131652924715,
            "btc_rsi_14": 33.44840837091841,
            "eth_price_usd": 3177.25,
            "eth_change_24h": 2.95,
            "eth_market_cap_percentage": 11.43216612211846,
            "sol_price_usd": 141.15,
            "sol_change_24h": 3.24,
            "xrp_price_usd": 2.2593,
            "xrp_change_24h": 0.071,
            "ada_price_usd": 0.5071,
            "ada_change_24h": 0.795,
            "link_price_usd": 14.2,
            "link_change_24h": 1.646,
            "bnb_price_usd": 935.51,
            "bnb_change_24h": 4.13,
            "market_cap_usd": 3334519158862.682,
            "volume_24h_usd": 208615359377.3596,
            "market_cap_change_percentage_24h_usd": 0.8706429089114247,
            "fng_value": 10,
            "us_stock_indices": {},
            "fetch_duration_ms": 114,
            "partial_failure": false,
            "last_updated": "2025-11-15T13:45:35.496238881+00:00",
            "timestamp": "2025-11-15T13:45:35.496253484+00:00"
        }"#;

        // Create payload from Redis JSON
        let payload = DashboardUpdatePayload::from_json_str(redis_json, "external_apis").unwrap();

        // Verify source
        assert_eq!(payload.source, "external_apis");
        assert_eq!(payload.data.btc_price_usd, 96062.47);

        // Wrap in ServerMessage and serialize
        let msg = ServerMessage::DashboardUpdate(payload);
        let json = msg.to_json_string().unwrap();

        // Should be camelCase for frontend
        assert!(json.contains("btcPriceUsd"));
        assert!(json.contains("external_apis"));
    }
}
