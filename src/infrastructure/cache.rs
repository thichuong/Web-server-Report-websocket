use anyhow::Result;
use std::ops::Deref;
use std::sync::Arc;

pub use multi_tier_cache::{
    CacheManager, CacheManagerStats, CacheStrategy, CacheSystem as LibraryCacheSystem,
};

/// Helper: return a realtime cache strategy with a 5 second TTL.
#[must_use]
pub fn realtime_strategy() -> CacheStrategy {
    CacheStrategy::Custom(std::time::Duration::from_secs(5))
}

/// Cache System - Two-tier caching system
pub struct CacheSystem(LibraryCacheSystem);

impl Deref for CacheSystem {
    type Target = LibraryCacheSystem;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CacheSystem {
    /// Initializes a new `CacheSystem`.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying multi-tier cache system fails to initialize
    /// (e.g., Redis connection failure).
    pub async fn new() -> Result<Self> {
        let inner = LibraryCacheSystem::new()
            .await
            .map_err(anyhow::Error::from)?;
        Ok(Self(inner))
    }

    pub async fn health_check(&self) -> bool {
        self.0.health_check().await
    }

    #[must_use]
    pub fn cache_manager(&self) -> &Arc<CacheManager> {
        &self.0.cache_manager
    }
}
