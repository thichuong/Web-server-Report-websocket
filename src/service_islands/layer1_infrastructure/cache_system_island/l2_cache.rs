//! L2 Cache - Redis Cache
//!
//! This module now re-exports from the multi-tier-cache library.
//! All L2 cache functionality (including Redis Streams) is provided by the external library.

// Re-export L2Cache from the library
#[allow(unused_imports)]
pub use multi_tier_cache::L2Cache;
