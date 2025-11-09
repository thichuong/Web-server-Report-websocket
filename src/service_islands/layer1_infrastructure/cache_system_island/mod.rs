//! Cache System Island (Library Wrapper)
//!
//! This module now wraps the multi-tier-cache library for backward compatibility.
//! All cache functionality is provided by the external library.
//!
//! Maintains compatibility with existing API aggregator interface.

use std::sync::Arc;
use anyhow::Result;

// Import and re-export from multi-tier-cache library
pub use multi_tier_cache::{
    CacheSystem as LibraryCacheSystem,
    CacheManager,
    CacheStrategy,
    L1Cache,
    L2Cache,
};

// Re-export stats struct for backward compatibility if needed
#[allow(unused_imports)]
pub use multi_tier_cache::CacheManagerStats;

// Module declarations (now just re-export files that themselves re-export from library)
pub mod l1_cache;
pub mod l2_cache;
pub mod cache_manager;

/// Cache System Island - Two-tier caching system
///
/// Now wraps the multi-tier-cache library for backward compatibility.
pub struct CacheSystemIsland {
    /// Internal library cache system
    inner: LibraryCacheSystem,
    /// Cache Manager - Unified cache operations (exposed for compatibility)
    pub cache_manager: Arc<CacheManager>,
    /// L1 Cache - Moka in-memory cache (exposed for compatibility)
    #[allow(dead_code)]
    pub l1_cache: Arc<L1Cache>,
    /// L2 Cache - Redis cache (exposed for compatibility)
    #[allow(dead_code)]
    pub l2_cache: Arc<L2Cache>,
}

impl CacheSystemIsland {
    /// Initialize the Cache System Island
    ///
    /// Now uses the multi-tier-cache library internally.
    pub async fn new() -> Result<Self> {
        println!("🏗️ Initializing Cache System Island (using multi-tier-cache library)...");

        // Initialize from library
        let inner = LibraryCacheSystem::new().await?;

        // Extract Arc references for backward compatibility
        let cache_manager = inner.cache_manager.clone();
        let l1_cache = inner.l1_cache.clone();
        let l2_cache = inner.l2_cache.clone();

        println!("✅ Cache System Island initialized successfully (library-backed)");

        Ok(Self {
            inner,
            cache_manager,
            l1_cache,
            l2_cache,
        })
    }

    /// Health check for cache system
    pub async fn health_check(&self) -> bool {
        self.inner.health_check().await
    }

    /// Get cache manager (for compatibility with existing code)
    #[allow(dead_code)]
    pub fn get_cache_manager(&self) -> Arc<CacheManager> {
        self.cache_manager.clone()
    }

    /// Direct access to cache manager
    pub fn cache_manager(&self) -> &Arc<CacheManager> {
        &self.cache_manager
    }
}
