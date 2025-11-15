//! Cache System Island (Library Wrapper)
//!
//! This module now wraps the multi-tier-cache library using the Deref pattern.
//! All cache functionality is provided by the external library with zero-cost abstraction.

use std::sync::Arc;
use std::ops::Deref;
use anyhow::Result;

// Import and re-export from multi-tier-cache library
pub use multi_tier_cache::{
    CacheSystem as LibraryCacheSystem,
    CacheManager,
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
/// Wraps the multi-tier-cache library using Deref for zero-cost access.
pub struct CacheSystemIsland(LibraryCacheSystem);

impl Deref for CacheSystemIsland {
    type Target = LibraryCacheSystem;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CacheSystemIsland {
    /// Initialize the Cache System Island
    ///
    /// Now uses the multi-tier-cache library internally.
    pub async fn new() -> Result<Self> {
        println!("🏗️ Initializing Cache System Island (using multi-tier-cache library)...");

        // Initialize from library
        let inner = LibraryCacheSystem::new().await?;

        println!("✅ Cache System Island initialized successfully (library-backed)");

        Ok(Self(inner))
    }

    /// Health check for cache system
    pub async fn health_check(&self) -> bool {
        self.0.health_check().await
    }

    /// Direct access to cache manager (idiomatic accessor)
    pub fn cache_manager(&self) -> &Arc<CacheManager> {
        &self.0.cache_manager
    }
}
