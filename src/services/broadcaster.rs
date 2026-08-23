//! WebSocket Broadcaster Module
//!
//! Provides a centralized broadcast channel for streaming real-time market data
//! to connected WebSocket clients.

use tokio::sync::broadcast;
use tracing::debug;

/// WebSocket Broadcaster
///
/// Manages the broadcast channel for distributing server messages to connected WebSocket clients.
pub struct Broadcaster {
    sender: broadcast::Sender<String>,
}

impl Broadcaster {
    /// Creates a new `Broadcaster` with the specified buffer capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Broadcasts a message to all active subscribers.
    ///
    /// Returns the number of receivers that received the message.
    /// If there are no active receivers, this returns 0 without error.
    pub fn broadcast(&self, message: &str) -> usize {
        match self.sender.send(message.to_string()) {
            Ok(receiver_count) => {
                debug!("Broadcasted message to {receiver_count} clients");
                receiver_count
            }
            Err(_) => {
                // No active receivers is normal when no clients are connected
                0
            }
        }
    }

    /// Subscribes to receive broadcasted messages.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    /// Checks if the broadcaster is operational.
    #[must_use]
    pub fn health_check(&self) -> bool {
        // Broadcast channel is always operational as long as sender exists
        true
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcaster_subscribe_and_broadcast() {
        let broadcaster = Broadcaster::new(100);
        let mut rx = broadcaster.subscribe();

        let count = broadcaster.broadcast("test_message");
        assert_eq!(count, 1);

        let received = rx.recv().await;
        assert_eq!(received.ok(), Some("test_message".to_string()));
    }

    #[test]
    fn test_broadcaster_no_subscribers() {
        let broadcaster = Broadcaster::new(100);
        let count = broadcaster.broadcast("no_one_listening");
        assert_eq!(count, 0);
    }
}
