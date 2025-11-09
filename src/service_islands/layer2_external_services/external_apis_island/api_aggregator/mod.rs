//! API Aggregator Component
//!
//! This component aggregates data from multiple APIs and handles coordination between different data sources.
//! The implementation is split across multiple modules for better maintainability:
//! - aggregator_core: Core struct and constructors
//! - dashboard_aggregator: Dashboard data aggregation logic
//! - crypto_fetchers: Cryptocurrency price fetching with caching
//! - market_fetchers: Market data fetching (global, FNG, RSI, indices) with caching

pub mod aggregator_core;
pub mod dashboard_aggregator;
pub mod crypto_fetchers;
pub mod market_fetchers;

// Re-export the main ApiAggregator struct
pub use aggregator_core::ApiAggregator;