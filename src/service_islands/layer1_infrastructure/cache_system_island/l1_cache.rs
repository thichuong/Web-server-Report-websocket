//! L1 Cache - Moka In-Memory Cache
//!
//! This module now re-exports from the multi-tier-cache library.
//! All L1 cache functionality is provided by the external library.

// Re-export L1Cache from the library
#[allow(unused_imports)]
pub use multi_tier_cache::L1Cache;

// Re-export CacheStats if needed for backward compatibility
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub coalesced_requests: u64,
    pub size: u64,
}
