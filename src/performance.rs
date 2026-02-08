//! Performance Optimization Module
//!
//! Provides optimized HTTP clients and performance utilities.

use reqwest::Client;
use std::sync::LazyLock;
use std::time::Duration;

/// Optimized HTTP client with connection pooling and timeouts
///
/// Falls back to a default client if the optimized configuration fails to build.
pub static OPTIMIZED_HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .pool_max_idle_per_host(10)
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|e| {
            eprintln!("⚠️ Failed to create optimized HTTP client: {e}, using default");
            Client::new()
        })
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_http_client_initialization() {
        // Ensure the client initializes without panicking
        let client = &*OPTIMIZED_HTTP_CLIENT;
        // We can't easily check internal configuration, but we can verify it's usable
        // (This just checks it doesn't crash on access)
        let _ = client; 
        assert!(true);
    }
}
