/// Distributed Coordination Module
///
/// This module provides distributed coordination primitives for multi-instance deployments.
///
/// # Features
/// - Leader Election: Ensures only one instance performs certain operations
/// - Redis-based distributed locking using SET NX EX pattern
/// - Automatic failover when leader instance crashes
///
/// # Use Cases
/// - API rate limit management (only leader fetches from external APIs)
/// - Background job scheduling (only leader runs periodic tasks)
/// - Cache warming (only leader pre-populates cache)
///
/// # Example
/// ```rust
/// use distributed_coordination::LeaderElectionService;
/// use std::sync::Arc;
/// use std::sync::atomic::AtomicBool;
///
/// // Create leader election service
/// let service = Arc::new(
///     LeaderElectionService::new("redis://localhost:6379", "node-1".to_string()).await?
/// );
///
/// // Create shared flag
/// let is_leader = Arc::new(AtomicBool::new(false));
///
/// // Start monitoring in background
/// tokio::spawn({
///     let service = service.clone();
///     let is_leader = is_leader.clone();
///     async move {
///         service.monitor_leadership(is_leader).await;
///     }
/// });
///
/// // In your application logic
/// if is_leader.load(Ordering::Relaxed) {
///     // Perform leader-only operations
///     fetch_api().await?;
/// }
/// ```

pub mod leader_election;

pub use leader_election::LeaderElectionService;
