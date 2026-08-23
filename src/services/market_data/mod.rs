//! Market Data Module
//!
//! Provides market data collection, real-time WebSocket feeds, REST fallbacks, and caching.

pub mod binance_ws;
pub mod http_client;
pub mod service;

pub use binance_ws::BinanceWsClient;
pub use http_client::{GlobalMarketMetrics, MarketDataHttpClient};
pub use service::MarketDataService;
