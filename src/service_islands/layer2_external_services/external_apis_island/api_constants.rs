// API Constants Component
//
// This module contains all API URL constants used by the market data API.

/// API URLs - extracted from existing data_service.rs with cache-friendly grouping

// Binance APIs (Primary)
// Multi-symbol endpoint - fetches all crypto prices in a single request (OPTIMIZED)
pub const BINANCE_MULTI_PRICE_URL: &str = r#"https://api.binance.com/api/v3/ticker/24hr?symbols=["BTCUSDT","ETHUSDT","SOLUSDT","XRPUSDT","ADAUSDT","LINKUSDT","BNBUSDT"]"#; // 10 sec cache (RealTime)

// CoinGecko APIs (Fallback)
pub const BASE_GLOBAL_URL: &str = "https://api.coingecko.com/api/v3/global"; // 30 sec cache

// CoinMarketCap APIs (Fallback)
pub const CMC_GLOBAL_URL: &str = "https://pro-api.coinmarketcap.com/v1/global-metrics/quotes/latest"; // 30 sec cache

// Other APIs
pub const BASE_FNG_URL: &str = "https://api.alternative.me/fng/?limit=1"; // 5 min cache
pub const BASE_RSI_URL_TEMPLATE: &str = "https://api.taapi.io/rsi?secret={secret}&exchange=binance&symbol=BTC/USDT&interval=1d"; // 5 min cache