// Data Models Component
//
// This module contains all data structures used by the market data API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// CoinGecko response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CoinGeckoGlobal {
    pub data: CoinGeckoGlobalData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CoinGeckoGlobalData {
    pub total_market_cap: HashMap<String, f64>,
    pub total_volume: HashMap<String, f64>,
    pub market_cap_change_percentage_24h_usd: f64,
    pub market_cap_percentage: HashMap<String, f64>,
}

// Binance response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BinanceBtcPrice {
    #[allow(dead_code)]
    pub symbol: String,
    #[serde(rename = "lastPrice")]
    pub last_price: String,
    #[serde(rename = "priceChangePercent")]
    pub price_change_percent: String,
}

// Binance Multi-Ticker response (array of tickers)
pub(crate) type BinanceMultiTickerResponse = Vec<BinanceBtcPrice>;

// Fear & Greed Index response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FearGreedResponse {
    pub data: Vec<FearGreedData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FearGreedData {
    pub value: String,
}

// TAAPI RSI response structures
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct TaapiRsiResponse {
    pub value: f64,
}

// CoinMarketCap response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CmcGlobalResponse {
    pub data: CmcGlobalData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CmcGlobalData {
    pub quote: HashMap<String, CmcGlobalQuote>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct CmcGlobalQuote {
    pub total_market_cap: f64,
    pub total_volume_24h: f64,
    pub market_cap_change_percentage_24h: f64,
    pub btc_dominance: f64,
    pub eth_dominance: f64,
}
