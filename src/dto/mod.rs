/// Data Transfer Objects (DTOs) for WebSocket communication
///
/// This module defines the API contract between the client and server,
/// providing type-safe message structures for bidirectional communication.
pub mod websocket;

// Re-export commonly used types
#[allow(unused_imports)]
pub use websocket::{
    CryptoPrice, DashboardData, DashboardUpdatePayload, ServerMessage,
};
