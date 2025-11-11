use anyhow::{Context, Result};
use redis::{AsyncCommands, Client};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

/// Leader Election Service using Redis distributed locking
///
/// This service implements a distributed leader election pattern using Redis SET NX EX.
/// Only one instance across all nodes will be the leader at any time.
///
/// # How it works:
/// - Leader acquires a Redis lock with TTL (e.g., 10 seconds)
/// - Leader renews the lock via heartbeat every 5 seconds
/// - If leader crashes, lock expires after TTL and another node becomes leader
/// - Followers continuously try to acquire leadership (every 5 seconds)
///
/// # Failover:
/// - Maximum failover time: TTL duration (10 seconds)
/// - Typical failover time: 5-8 seconds
pub struct LeaderElectionService {
    /// Redis client for distributed locking
    redis_client: Client,

    /// Unique identifier for this node
    node_id: String,

    /// Redis key for the leader election lock
    election_key: String,

    /// How often to check/renew leadership (seconds)
    heartbeat_interval: Duration,

    /// How long the lock is valid (seconds)
    lock_ttl: Duration,
}

impl LeaderElectionService {
    /// Create a new leader election service
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL (e.g., "redis://127.0.0.1:6379")
    /// * `node_id` - Unique identifier for this node instance
    ///
    /// # Example
    /// ```
    /// let service = LeaderElectionService::new(
    ///     "redis://localhost:6379",
    ///     "ws-instance-1".to_string()
    /// ).await?;
    /// ```
    pub async fn new(redis_url: &str, node_id: String) -> Result<Self> {
        let redis_client = Client::open(redis_url)
            .context("Failed to create Redis client for leader election")?;

        // Test connection
        let mut conn = redis_client
            .get_multiplexed_async_connection()
            .await
            .context("Failed to connect to Redis for leader election")?;

        // Ping to verify connection
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .context("Failed to ping Redis")?;

        info!(
            "Leader election service initialized for node: {}",
            node_id
        );

        Ok(Self {
            redis_client,
            node_id,
            election_key: "websocket:leader".to_string(),
            heartbeat_interval: Duration::from_secs(5),
            lock_ttl: Duration::from_secs(10),
        })
    }

    /// Attempt to acquire leadership (non-blocking)
    ///
    /// Uses Redis SET NX EX command for atomic lock acquisition with TTL.
    /// Returns true if this node successfully acquired leadership.
    ///
    /// # Example
    /// ```
    /// if service.try_acquire_leadership().await? {
    ///     println!("I am the leader!");
    /// }
    /// ```
    pub async fn try_acquire_leadership(&self) -> Result<bool> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .context("Failed to get Redis connection")?;

        // SET key value NX EX seconds
        // NX = Only set if key doesn't exist
        // EX = Set expiration time in seconds
        let result: Option<String> = redis::cmd("SET")
            .arg(&self.election_key)
            .arg(&self.node_id)
            .arg("NX") // Only set if not exists
            .arg("EX") // Set expiration
            .arg(self.lock_ttl.as_secs())
            .query_async(&mut conn)
            .await
            .context("Failed to execute SET NX EX command")?;

        let acquired = result.is_some();

        if acquired {
            info!("🎖️  Node {} acquired LEADERSHIP", self.node_id);
        } else {
            debug!("Node {} failed to acquire leadership (another node is leader)", self.node_id);
        }

        Ok(acquired)
    }

    /// Check if this node is currently the leader
    ///
    /// Returns true if the lock is held by this node.
    pub async fn is_leader(&self) -> Result<bool> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .context("Failed to get Redis connection")?;

        let current_leader: Option<String> = conn
            .get(&self.election_key)
            .await
            .context("Failed to get leader from Redis")?;

        Ok(current_leader.as_deref() == Some(self.node_id.as_str()))
    }

    /// Renew leadership (heartbeat)
    ///
    /// Extends the lock TTL if this node is still the leader.
    /// Uses Lua script for atomic check-and-extend operation.
    ///
    /// Returns true if leadership was successfully renewed.
    pub async fn renew_leadership(&self) -> Result<bool> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .context("Failed to get Redis connection")?;

        // Lua script for atomic check-and-renew
        // Only extend TTL if we're still the owner
        let script = redis::Script::new(
            r#"
            if redis.call("GET", KEYS[1]) == ARGV[1] then
                return redis.call("EXPIRE", KEYS[1], ARGV[2])
            else
                return 0
            end
            "#,
        );

        let result: i32 = script
            .key(&self.election_key)
            .arg(&self.node_id)
            .arg(self.lock_ttl.as_secs())
            .invoke_async(&mut conn)
            .await
            .context("Failed to renew leadership")?;

        let renewed = result == 1;

        if renewed {
            debug!("♻️  Node {} renewed leadership", self.node_id);
        } else {
            warn!("⚠️  Node {} lost leadership (cannot renew)", self.node_id);
        }

        Ok(renewed)
    }

    /// Release leadership (graceful shutdown)
    ///
    /// Deletes the lock if this node is the owner.
    /// Use this during graceful shutdown to allow faster failover.
    pub async fn release_leadership(&self) -> Result<()> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .context("Failed to get Redis connection")?;

        // Lua script for atomic check-and-delete
        // Only delete if we own the lock
        let script = redis::Script::new(
            r#"
            if redis.call("GET", KEYS[1]) == ARGV[1] then
                return redis.call("DEL", KEYS[1])
            else
                return 0
            end
            "#,
        );

        let result: i32 = script
            .key(&self.election_key)
            .arg(&self.node_id)
            .invoke_async(&mut conn)
            .await
            .context("Failed to release leadership")?;

        if result == 1 {
            info!("🔓 Node {} released leadership", self.node_id);
        } else {
            debug!("Node {} was not leader (nothing to release)", self.node_id);
        }

        Ok(())
    }

    /// Start leadership monitoring loop
    ///
    /// This spawns a background task that:
    /// - Tries to acquire leadership every heartbeat_interval
    /// - If leader, renews the lock periodically
    /// - Updates the is_leader_flag atomically
    ///
    /// # Arguments
    /// * `is_leader_flag` - Shared atomic boolean that tracks leadership status
    ///
    /// # Example
    /// ```
    /// let is_leader = Arc::new(AtomicBool::new(false));
    /// let service = Arc::new(LeaderElectionService::new(...).await?);
    ///
    /// tokio::spawn({
    ///     let service = service.clone();
    ///     let is_leader = is_leader.clone();
    ///     async move {
    ///         service.monitor_leadership(is_leader).await;
    ///     }
    /// });
    /// ```
    pub async fn monitor_leadership(self: Arc<Self>, is_leader_flag: Arc<AtomicBool>) {
        info!(
            "🔍 Starting leadership monitoring for node: {}",
            self.node_id
        );

        let mut interval = time::interval(self.heartbeat_interval);

        loop {
            interval.tick().await;

            let was_leader = is_leader_flag.load(Ordering::Relaxed);

            // Try to acquire or renew leadership
            let is_leader = if was_leader {
                // Already leader - try to renew
                match self.renew_leadership().await {
                    Ok(renewed) => renewed,
                    Err(e) => {
                        error!("❌ Failed to renew leadership: {}", e);
                        false
                    }
                }
            } else {
                // Not leader - try to acquire
                match self.try_acquire_leadership().await {
                    Ok(acquired) => acquired,
                    Err(e) => {
                        error!("❌ Failed to acquire leadership: {}", e);
                        false
                    }
                }
            };

            // Update flag atomically
            is_leader_flag.store(is_leader, Ordering::Relaxed);

            // Log leadership changes
            if is_leader && !was_leader {
                info!(
                    "🎖️  LEADERSHIP ACQUIRED - Node {} is now the LEADER",
                    self.node_id
                );
            } else if !is_leader && was_leader {
                warn!(
                    "🔄 LEADERSHIP LOST - Node {} is now a FOLLOWER",
                    self.node_id
                );
            }
        }
    }

    /// Get the node ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Get the election key name
    pub fn election_key(&self) -> &str {
        &self.election_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Redis running
    async fn test_leader_election() {
        let service = LeaderElectionService::new(
            "redis://127.0.0.1:6379",
            "test-node-1".to_string(),
        )
        .await
        .unwrap();

        // First attempt should acquire leadership
        assert!(service.try_acquire_leadership().await.unwrap());

        // Should be leader
        assert!(service.is_leader().await.unwrap());

        // Should be able to renew
        assert!(service.renew_leadership().await.unwrap());

        // Release leadership
        service.release_leadership().await.unwrap();

        // Should no longer be leader
        assert!(!service.is_leader().await.unwrap());
    }
}
