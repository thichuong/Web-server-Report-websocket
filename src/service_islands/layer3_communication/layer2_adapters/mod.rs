//! Layer 2 Adapters - Layer 3 to Layer 2 Communication Bridge (gRPC-Based)
//!
//! This module contains all adapter functions that Layer 3 uses to communicate
//! with Layer 2 Data Service via gRPC. It serves as a clean abstraction layer
//! maintaining the Service Islands Architecture dependency flow:
//! Layer 3 → Layer 2 (via gRPC)
//!
//! All Layer 2 communication goes through these adapters to maintain
//! proper separation of concerns and make Layer 2 calls easy to manage.

pub mod market_data_adapter;
pub mod api_aggregator_adapter;

pub use market_data_adapter::MarketDataAdapter;
pub use api_aggregator_adapter::ApiAggregatorAdapter;

use anyhow::Result;
use std::sync::Arc;

/// Layer 2 Adapters Hub
/// 
/// Central coordinator for all Layer 2 communication adapters.
/// Provides a unified interface for Layer 3 to access Layer 2 services.
pub struct Layer2AdaptersHub {
    /// Market data fetching adapter
    pub market_data: MarketDataAdapter,
    /// API aggregation adapter  
    pub api_aggregator: ApiAggregatorAdapter,
}

impl Layer2AdaptersHub {
    /// Initialize all Layer 2 adapters
    pub fn new() -> Self {
        println!("🔧 Initializing Layer 2 Adapters Hub...");
        
        let market_data = MarketDataAdapter::new();
        let api_aggregator = ApiAggregatorAdapter::new();
        
        println!("✅ Layer 2 Adapters Hub initialized successfully");
        
        Self {
            market_data,
            api_aggregator,
        }
    }
    
    /// Initialize Layer 2 adapters with gRPC Client dependency (High Performance)
    ///
    /// Note: Not used in websocket service - commented out as Layer2GrpcClient doesn't exist here
    // #[allow(dead_code)]
    // pub fn with_layer2_grpc_client(mut self, _layer2_grpc_client: Arc<String>) -> Self {
    //     println!("🚀 gRPC not used in websocket service");
    //     self
    // }
    
    /// Initialize Layer 2 adapters with Layer 1 Cache System (OPTIMIZATION)
    /// 
    /// Enables Layer 3 cache optimization by providing direct access to cache system.
    /// This allows adapters to check cache before calling Layer 2.
    pub fn with_cache_system(mut self, cache_system: Arc<crate::service_islands::layer1_infrastructure::cache_system_island::CacheSystemIsland>) -> Self {
        println!("🔗 Connecting Layer 2 Adapters Hub to Cache System (Layer 3 optimization)...");
        
        // Note: Arc::clone retained for future extensibility
        // Additional adapters may need cache_system access (see comment below)
        self.market_data = self.market_data.with_cache_system(Arc::clone(&cache_system));
        // Additional adapters can be connected to cache system here in the future
        // e.g., self.api_aggregator = self.api_aggregator.with_cache_system(Arc::clone(&cache_system));
        
        println!("✅ Layer 2 Adapters Hub connected to Cache System - Layer 3 cache optimization enabled");
        
        self
    }
    
    /// Health check for all adapters
    pub async fn health_check(&self) -> Result<()> {
        println!("🏥 Checking Layer 2 Adapters Hub health...");
        
        // Check all adapter components
        let checks = vec![
            ("Market Data Adapter", self.market_data.health_check().await),
            ("API Aggregator Adapter", self.api_aggregator.health_check().await),
        ];
        
        let mut all_healthy = true;
        for (adapter, healthy) in checks {
            if healthy {
                println!("  ✅ {} - Healthy", adapter);
            } else {
                println!("  ❌ {} - Unhealthy", adapter);
                all_healthy = false;
            }
        }
        
        if all_healthy {
            println!("✅ Layer 2 Adapters Hub - All adapters healthy");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Layer 2 Adapters Hub - Some adapters unhealthy"))
        }
    }
}
