//! Structured Error Types for Service Islands
//!
//! This module defines all error types using `thiserror` for type-safe error handling.
//! Following Rust best practices: library code uses `thiserror`, application code uses `anyhow`.

use thiserror::Error;

/// Errors that can occur when interacting with external APIs
#[derive(Error, Debug)]
pub enum ExternalApiError {
    #[error("API request failed for {api}: {source}")]
    RequestFailed {
        api: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("API rate limit exceeded for {api}: {details}")]
    RateLimited { api: String, details: String },

    #[error("Invalid API response from {api}: {reason}")]
    InvalidResponse { api: String, reason: String },

    #[error("API timeout for {api} after {timeout_secs}s")]
    Timeout { api: String, timeout_secs: u64 },

    #[error("Missing required API key: {key_name}")]
    MissingApiKey { key_name: String },

    #[error("API authentication failed for {api}")]
    AuthenticationFailed { api: String },
}

/// Errors that can occur during data aggregation
#[derive(Error, Debug)]
pub enum AggregationError {
    #[error("Failed to fetch crypto prices: {0}")]
    CryptoFetchFailed(#[from] ExternalApiError),

    #[error("Failed to parse {field} from API response: {reason}")]
    ParseError { field: String, reason: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("All data sources failed, cannot aggregate")]
    AllSourcesFailed,

    #[error("Partial aggregation failure: {successful}/{total} sources succeeded")]
    PartialFailure { successful: usize, total: usize },

    #[error("Data deserialization failed: {0}")]
    DeserializationFailed(#[from] serde_json::Error),
}

/// Errors that can occur with cache operations
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache read failed for key '{key}': {reason}")]
    ReadFailed { key: String, reason: String },

    #[error("Cache write failed for key '{key}': {reason}")]
    WriteFailed { key: String, reason: String },

    #[error("Cache serialization failed: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("Cache connection lost")]
    ConnectionLost,

    #[error("Cache key not found: {key}")]
    KeyNotFound { key: String },
}

/// Errors that can occur during data processing
#[derive(Error, Debug)]
pub enum DataProcessingError {
    #[error("Invalid data format for {field}: expected {expected}, got {got}")]
    InvalidFormat {
        field: String,
        expected: String,
        got: String,
    },

    #[error("Data validation failed for {field}: {reason}")]
    ValidationFailed { field: String, reason: String },

    #[error("Numeric conversion failed for {field}: {value}")]
    NumericConversionFailed { field: String, value: String },

    #[error("Missing required data: {0}")]
    MissingData(String),
}

/// Conversion from reqwest::Error to ExternalApiError
impl From<reqwest::Error> for ExternalApiError {
    fn from(err: reqwest::Error) -> Self {
        ExternalApiError::RequestFailed {
            api: "unknown".to_string(),
            source: err,
        }
    }
}

/// Helper function to create parse errors
pub fn parse_error(field: impl Into<String>, reason: impl Into<String>) -> AggregationError {
    AggregationError::ParseError {
        field: field.into(),
        reason: reason.into(),
    }
}

/// Helper function to create missing field errors
pub fn missing_field(field: impl Into<String>) -> AggregationError {
    AggregationError::MissingField {
        field: field.into(),
    }
}
