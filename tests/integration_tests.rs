#![allow(
    clippy::expect_used,
    clippy::unreadable_literal,
    clippy::default_trait_access
)]

use web_server_report_websocket::dto::websocket::{DashboardData, ServerMessage};

#[test]
fn test_dashboard_data_structure() {
    // This integration test verifies that the public API (DTOs) behaves as expected
    // It imports from the library crate

    let data = DashboardData {
        btc_price_usd: 100000.0,
        btc_change_24h: 5.0,
        btc_market_cap_percentage: 60.0,
        btc_rsi_14: 50.0,
        eth_price_usd: 4000.0,
        eth_change_24h: 2.0,
        eth_market_cap_percentage: 15.0,
        sol_price_usd: 150.0,
        sol_change_24h: 3.0,
        xrp_price_usd: 1.0,
        xrp_change_24h: 0.5,
        ada_price_usd: 0.5,
        ada_change_24h: 1.0,
        link_price_usd: 20.0,
        link_change_24h: 1.5,
        bnb_price_usd: 400.0,
        bnb_change_24h: 1.0,
        market_cap_usd: 3_000_000_000_000.0,
        volume_24h_usd: 100_000_000_000.0,
        market_cap_change_percentage_24h_usd: 2.0,
        fng_value: 75,
        us_stock_indices: Default::default(),
        fetch_duration_ms: 200,
        partial_failure: false,
        last_updated: "2024-01-01T00:00:00Z".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
    };

    let payload = web_server_report_websocket::dto::websocket::DashboardUpdatePayload::new(
        data,
        "integration_test",
    );
    let msg = ServerMessage::DashboardUpdate(Box::new(payload));

    let json = msg.to_json_string().expect("Serialization should succeed");
    assert!(json.contains("DashboardUpdate"));
    assert!(json.contains("btc_price_usd"));
}
