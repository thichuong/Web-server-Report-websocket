//! Cache Manager - Unified Cache Operations
//!
//! This module now re-exports from the multi-tier-cache library.
//! All cache management functionality is provided by the external library.

// Re-export from the library
#[allow(unused_imports)]
pub use multi_tier_cache::{CacheManager, CacheStrategy, CacheManagerStats};

/// Helper: return a realtime cache strategy with a 2 second TTL.
///
/// Use this when you specifically want a 2s realtime TTL instead of
/// the library's `CacheStrategy::RealTime` default.
pub fn realtime_strategy() -> CacheStrategy {
	CacheStrategy::Custom(std::time::Duration::from_secs(2))
}
