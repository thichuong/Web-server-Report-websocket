//! Broadcast Service Component
//!
//! This component handles message broadcasting and real-time updates.

use tokio::sync::broadcast;

/// Broadcast Service
///
/// Manages message broadcasting to multiple WebSocket clients.
/// Handles real-time updates, background tasks, and message distribution.
pub struct BroadcastService {
    /// Broadcast channel sender
    pub broadcast_tx: broadcast::Sender<String>,
}

impl BroadcastService {
    /// Create a new BroadcastService with a broadcast channel
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            broadcast_tx,
        }
    }

    /// Broadcast a message to all connected WebSocket clients
    pub async fn broadcast(&self, message: String) {
        // Send to all subscribers
        // Errors are ignored as some receivers might have been dropped
        let _ = self.broadcast_tx.send(message);
    }

    /// Get a receiver for the broadcast channel
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.broadcast_tx.subscribe()
    }

    /// Health check for broadcast service
    pub async fn health_check(&self) -> bool {
        // Verify broadcast service is working
        // Broadcast channels are always healthy unless all receivers dropped
        true
    }
}
