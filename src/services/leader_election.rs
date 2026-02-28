use anyhow::{Context, Result};
use redis::Client;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

/// Leader Election Service using Redis distributed locking
pub struct LeaderElectionService {
    redis_client: Client,
    node_id: String,
    election_key: String,
    heartbeat_interval: Duration,
    lock_ttl: Duration,
}

impl LeaderElectionService {
    pub async fn new(redis_url: &str, node_id: String) -> Result<Self> {
        let redis_client = Client::open(redis_url).context("Failed to create Redis client for leader election")?;
        let mut conn = redis_client.get_multiplexed_async_connection().await.context("Failed to connect to Redis for leader election")?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await.context("Failed to ping Redis")?;
        info!("Leader election service initialized for node: {}", node_id);
        Ok(Self {
            redis_client,
            node_id,
            election_key: "websocket:leader".to_string(),
            heartbeat_interval: Duration::from_secs(5),
            lock_ttl: Duration::from_secs(10),
        })
    }

    pub async fn try_acquire_leadership(&self) -> Result<bool> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await.context("Failed to get Redis connection")?;
        let result: Option<String> = redis::cmd("SET")
            .arg(&self.election_key)
            .arg(&self.node_id)
            .arg("NX")
            .arg("EX")
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

    pub async fn renew_leadership(&self) -> Result<bool> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await.context("Failed to get Redis connection")?;
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

    pub async fn release_leadership(&self) -> Result<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await.context("Failed to get Redis connection")?;
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

    pub async fn monitor_leadership(self: Arc<Self>, is_leader_flag: Arc<AtomicBool>) {
        info!("🔍 Starting leadership monitoring for node: {}", self.node_id);
        let mut interval = time::interval(self.heartbeat_interval);
        loop {
            interval.tick().await;
            let was_leader = is_leader_flag.load(Ordering::Relaxed);
            let is_leader = if was_leader {
                match self.renew_leadership().await {
                    Ok(renewed) => renewed,
                    Err(e) => {
                        error!("❌ Failed to renew leadership: {}", e);
                        false
                    }
                }
            } else {
                match self.try_acquire_leadership().await {
                    Ok(acquired) => acquired,
                    Err(e) => {
                        error!("❌ Failed to acquire leadership: {}", e);
                        false
                    }
                }
            };
            is_leader_flag.store(is_leader, Ordering::Relaxed);
            if is_leader && !was_leader {
                info!("🎖️  LEADERSHIP ACQUIRED - Node {} is now the LEADER", self.node_id);
            } else if !is_leader && was_leader {
                warn!("🔄 LEADERSHIP LOST - Node {} is now a FOLLOWER", self.node_id);
            }
        }
    }
}
