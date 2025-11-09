//! API Aggregator Adapter - Layer 3 to Layer 2 API Aggregation Bridge
//! 
//! This adapter handles all API aggregation operations from Layer 2.
//! It provides a clean abstraction for Layer 3 components to access
//! Layer 2 External APIs Island aggregation services.

use anyhow::Result;
use serde_json;
use std::sync::Arc;

use crate::service_islands::layer2_external_services::external_apis_island::ExternalApisIsland;

/// API Aggregator Adapter
/// 
/// Handles all Layer 3 → Layer 2 API aggregation communication.
/// Provides methods for accessing aggregated API data services
/// while maintaining proper Service Islands Architecture.
pub struct ApiAggregatorAdapter {
    /// Reference to Layer 2 External APIs Island
    external_apis: Option<Arc<ExternalApisIsland>>,
}

#[allow(dead_code)]
impl ApiAggregatorAdapter {
    /// Create new API Aggregator Adapter without Layer 2 dependency
    pub fn new() -> Self {
        Self {
            external_apis: None,
        }
    }
    
    /// Set Layer 2 External APIs dependency
    pub fn with_external_apis(mut self, external_apis: Arc<ExternalApisIsland>) -> Self {
        self.external_apis = Some(external_apis);
        self
    }
    
    /// Fetch aggregated market statistics from Layer 2
    /// 
    /// Gets comprehensive market statistics from multiple API sources.
    pub async fn fetch_market_statistics(&self) -> Result<serde_json::Value> {
        if let Some(external_apis) = &self.external_apis {
            println!("🔄 [Layer 3 → Layer 2] Fetching aggregated market statistics...");
            // This would call a market statistics aggregation method
            external_apis.fetch_dashboard_summary_v2(false).await
        } else {
            Err(anyhow::anyhow!("Layer 2 External APIs not configured in ApiAggregatorAdapter"))
        }
    }
    
    /// Fetch multiple crypto currencies data from Layer 2
    /// 
    /// Gets data for multiple cryptocurrencies in a single call.
    pub async fn fetch_multi_crypto_data(&self, symbols: Vec<&str>) -> Result<serde_json::Value> {
        if let Some(_external_apis) = &self.external_apis {
            println!("🔄 [Layer 3 → Layer 2] Fetching multi-crypto data for: {:?}...", symbols);
            
            // For now, return a placeholder structure
            // In the future, this would aggregate data for multiple cryptocurrencies
            let multi_data = serde_json::json!({
                "symbols": symbols,
                "data": {},
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "source": "layer2_api_aggregator",
                "status": "placeholder"
            });
            
            Ok(multi_data)
        } else {
            Err(anyhow::anyhow!("Layer 2 External APIs not configured in ApiAggregatorAdapter"))
        }
    }
    
    /// Fetch market trends and analysis from Layer 2
    /// 
    /// Gets trend analysis data from multiple sources.
    pub async fn fetch_market_trends(&self) -> Result<serde_json::Value> {
        if let Some(_external_apis) = &self.external_apis {
            println!("🔄 [Layer 3 → Layer 2] Fetching market trends analysis...");
            
            // Placeholder for market trends aggregation
            let trends_data = serde_json::json!({
                "trends": {
                    "short_term": "bullish",
                    "medium_term": "neutral",
                    "long_term": "bullish"
                },
                "confidence": 0.75,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "source": "layer2_api_aggregator",
                "status": "placeholder"
            });
            
            Ok(trends_data)
        } else {
            Err(anyhow::anyhow!("Layer 2 External APIs not configured in ApiAggregatorAdapter"))
        }
    }
    
    /// Fetch news and sentiment data from Layer 2
    /// 
    /// Gets aggregated news and sentiment analysis.
    pub async fn fetch_news_sentiment(&self) -> Result<serde_json::Value> {
        if let Some(_external_apis) = &self.external_apis {
            println!("🔄 [Layer 3 → Layer 2] Fetching news and sentiment data...");
            
            // Placeholder for news and sentiment aggregation
            let news_data = serde_json::json!({
                "sentiment_score": 0.6,
                "news_count": 125,
                "top_keywords": ["bitcoin", "ethereum", "regulation", "adoption"],
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "source": "layer2_api_aggregator",
                "status": "placeholder"
            });
            
            Ok(news_data)
        } else {
            Err(anyhow::anyhow!("Layer 2 External APIs not configured in ApiAggregatorAdapter"))
        }
    }
    
    /// Health check for API aggregator adapter
    pub async fn health_check(&self) -> bool {
        if let Some(external_apis) = &self.external_apis {
            match external_apis.health_check().await {
                Ok(_) => {
                    println!("  ✅ API Aggregator Adapter - Layer 2 connection healthy");
                    true
                }
                Err(e) => {
                    // Be tolerant of rate limiting and temporary issues
                    let error_msg = e.to_string();
                    if error_msg.contains("429") || error_msg.contains("Circuit breaker") || error_msg.contains("rate limit") {
                        println!("  ⚠️ API Aggregator Adapter - Layer 2 rate limited (functional)");
                        true
                    } else {
                        println!("  ❌ API Aggregator Adapter - Layer 2 connection unhealthy: {}", e);
                        false
                    }
                }
            }
        } else {
            println!("  ⚠️ API Aggregator Adapter - Layer 2 not configured");
            true // Not configured is not an error
        }
    }
    
    /// Check if Layer 2 is configured
    pub fn is_layer2_configured(&self) -> bool {
        self.external_apis.is_some()
    }
}
