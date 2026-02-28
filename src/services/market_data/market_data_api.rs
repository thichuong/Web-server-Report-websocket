//! Market Data API Component
//!
//! This component handles direct interactions with cryptocurrency APIs.
//! Refactored into smaller modules for better maintainability.

// Include all modules
include!("api_constants.rs");
include!("models.rs");
include!("market_data_core.rs");
include!("crypto_fetchers.rs");
include!("market_fetchers.rs");
