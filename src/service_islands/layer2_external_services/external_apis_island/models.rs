// Data Models Component
//
// This module contains all data structures used by the market data API.

use serde::Deserialize;
use std::collections::HashMap;

// CoinGecko response structures
#[derive(Debug, Deserialize)]
pub(crate) struct CoinGeckoGlobal {
    pub data: CoinGeckoGlobalData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CoinGeckoGlobalData {
    pub total_market_cap: HashMap<String, f64>,
    pub total_volume: HashMap<String, f64>,
    pub market_cap_change_percentage_24h_usd: f64,
    pub market_cap_percentage: HashMap<String, f64>,
}

// Binance response structures
#[derive(Debug, Deserialize, serde::Serialize)]
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
#[derive(Debug, Deserialize)]
pub(crate) struct FearGreedResponse {
    pub data: Vec<FearGreedData>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FearGreedData {
    pub value: String,
}

// TAAPI RSI response structures
#[derive(Debug, Deserialize)]
pub(crate) struct TaapiRsiResponse {
    pub value: f64,
}

// CoinMarketCap response structures
#[derive(Debug, Deserialize)]
pub(crate) struct CmcGlobalResponse {
    pub data: CmcGlobalData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CmcGlobalData {
    pub quote: HashMap<String, CmcGlobalQuote>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CmcGlobalQuote {
    pub total_market_cap: f64,
    pub total_volume_24h: f64,
    pub market_cap_change_percentage_24h: f64,
    pub btc_dominance: f64,
    pub eth_dominance: f64,
}

// Finnhub response structures
#[derive(Debug, Deserialize)]
pub(crate) struct FinnhubQuoteResponse {
    #[serde(rename = "c")]
    pub current_price: f64,
    #[serde(rename = "d")]
    pub change: f64,
    #[serde(rename = "dp")]
    pub percent_change: f64,
    #[allow(dead_code)]
    #[serde(rename = "h")]
    pub high: f64,
    #[allow(dead_code)]
    #[serde(rename = "l")]
    pub low: f64,
    #[allow(dead_code)]
    #[serde(rename = "o")]
    pub open: f64,
    #[allow(dead_code)]
    #[serde(rename = "pc")]
    pub previous_close: f64,
}