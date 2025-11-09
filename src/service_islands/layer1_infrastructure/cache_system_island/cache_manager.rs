//! Cache Manager - Unified Cache Operations
//!
//! This module now re-exports from the multi-tier-cache library.
//! All cache management functionality is provided by the external library.

// Re-export from the library
#[allow(unused_imports)]
pub use multi_tier_cache::{CacheManager, CacheStrategy, CacheManagerStats};
