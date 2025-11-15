//! Performance Optimization Module
//!
//! Provides optimized HTTP clients and performance utilities.

use std::time::Duration;
use std::sync::LazyLock;
use reqwest::Client;

/// Optimized HTTP client with connection pooling and timeouts
///
/// Falls back to a default client if the optimized configuration fails to build.
pub static OPTIMIZED_HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|e| {
            eprintln!("⚠️ Failed to create optimized HTTP client: {}, using default", e);
            Client::new()
        })
});
