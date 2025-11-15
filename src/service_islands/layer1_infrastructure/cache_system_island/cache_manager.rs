//! Cache Manager - Unified Cache Operations
//!
//! This module now re-exports from the multi-tier-cache library.
//! All cache management functionality is provided by the external library.

// Re-export from the library
#[allow(unused_imports)]
pub use multi_tier_cache::{CacheManager, CacheStrategy, CacheManagerStats};

/// Helper: return a realtime cache strategy with a 5 second TTL.
///
/// Use this for real-time market data that updates frequently.
/// The 5-second TTL balances freshness with API rate limiting.
pub fn realtime_strategy() -> CacheStrategy {
	CacheStrategy::Custom(std::time::Duration::from_secs(5))
}
