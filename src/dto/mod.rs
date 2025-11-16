/// Data Transfer Objects (DTOs) for WebSocket communication
///
/// This module defines the API contract between the client and server,
/// providing type-safe message structures for bidirectional communication.

pub mod websocket;

// Re-export commonly used types
pub use websocket::{ClientMessage, ServerMessage, DashboardData, DashboardUpdatePayload};
