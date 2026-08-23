# System Architecture

## Overview

`web-server-report-websocket` is a high-performance Rust microservice responsible for real-time cryptocurrency market data broadcasting via WebSockets. It bridges external market data providers (Binance, CoinGecko, CoinMarketCap, Alternative.me, TAAPI) with frontend clients and consumer services, achieving sub-millisecond in-memory lookups and robust distributed coordination.

## Core Components

The application follows a streamlined, modular architecture divided into the following layers:

```
src/
├── api/                   # HTTP & WebSocket routing and request handlers
│   ├── handlers.rs        # WebSocket connection loop & /health handler
│   ├── routes.rs          # Axum router configuration
│   └── state.rs           # Shared AppState container
├── config/                # Environment configuration loading
│   └── app_env.rs         # Strongly-typed AppConfig
├── dto/                   # Shared DTOs and WebSocket message protocols
│   └── websocket.rs       # ServerMessage, DashboardData, CryptoPrice
├── infrastructure/        # Low-level cache adapters
│   └── cache.rs           # MultiTierCache (L1 Moka + L2 Redis)
├── performance.rs         # Shared connection-pooled HTTP client
└── services/              # Core business services
    ├── broadcaster.rs     # Broadcaster wrapping tokio::sync::broadcast
    ├── leader_election.rs # Distributed leader election via Redis locks
    └── market_data/       # Unified market data subsystem
        ├── binance_ws.rs  # Real-time Binance combined stream & in-memory store
        ├── http_client.rs # REST clients for Binance, CoinGecko, CMC, FNG, TAAPI
        └── service.rs     # MarketDataService orchestrator & cache integration
```

### 1. `api` Layer
- **`routes.rs`**: Configures Axum endpoints:
  - `GET /ws`: WebSocket endpoint for streaming real-time market updates.
  - `GET /health`: Detailed service health check.
- **`state.rs` (`AppState`)**: Holds shared application dependencies:
  - `cache: Arc<CacheSystem>`
  - `market_data: Arc<MarketDataService>`
  - `broadcaster: Arc<Broadcaster>`
  - `leader_election: Arc<LeaderElectionService>`
  - `is_leader: Arc<AtomicBool>`
  - `active_ws_connections: Arc<AtomicUsize>`

### 2. `services` Layer
- **`Broadcaster`**: Lightweight message broadcaster wrapping a `tokio::sync::broadcast::Sender<String>`. Provides non-blocking publish (`broadcast`) and client subscription (`subscribe`).
- **`LeaderElectionService`**: Implements distributed leader election using Redis `SET NX` locks with automatic TTL heartbeats, graceful release, and seamless failover.
- **`MarketDataService`**: Coordinates real-time market feeds and caching strategies:
  - **`BinanceWsClient`**: Connects to Binance Combined WebSocket streams (`btcusdt`, `ethusdt`, `solusdt`, `xrpusdt`, `adausdt`, `linkusdt`, `bnbusdt`), maintaining a low-latency thread-safe `DashMap` in-memory price cache. Automatically switches to Binance US if geoblocked and handles auto-reconnect with exponential backoff.
  - **`MarketDataHttpClient`**: Executes resilient HTTP REST calls for Binance multi-ticker fallback, CoinGecko global data (with CoinMarketCap fallback), Alternative.me Fear & Greed Index, and TAAPI BTC RSI-14.
  - **Aggregation Pipeline (`fetch_dashboard_data`)**: Concurrently fetches crypto prices, global market metrics, FNG, and RSI with timeouts and multi-tier cache strategies (RealTime 15s, ShortTerm 5m, LongTerm 1h/3h).

### 3. `infrastructure` Layer
- **`CacheSystem`**: Integrates `multi-tier-cache` providing L1 in-memory caching (Moka) and L2 distributed caching (Redis), optimizing network bandwidth and preventing external API rate limiting.

### 4. `dto` Layer
- **`websocket.rs`**: Defines serialization models (`DashboardData`, `CryptoPrice`, `ServerMessage`, `DashboardUpdatePayload`).

---

## Data Flow

```
                     ┌────────────────────────────────────────────────────────┐
                     │                    External Sources                    │
                     │  Binance WS (Primary) ──► Binance REST (Fallback)     │
                     │  CoinGecko ──► CoinMarketCap (Fallback)                │
                     │  Alternative.me (FNG)  ──► TAAPI.io (RSI)              │
                     └───────────────────────────┬────────────────────────────┘
                                                 │
                                                 ▼
                                     ┌───────────────────────┐
                                     │   MarketDataService   │
                                     │  (Aggregation Engine) │
                                     └───────────┬───────────┘
                                                 │
                     ┌───────────────────────────┴───────────────────────────┐
                     │                                                       │
                     ▼                                                       ▼
          ┌─────────────────────┐                                 ┌─────────────────────┐
          │  Multi-Tier Cache   │                                 │     Broadcaster     │
          │ (L1 Moka + L2 Redis)│                                 │ (broadcast channel) │
          └──────────┬──────────┘                                 └──────────┬──────────┘
                     │                                                       │
                     ▼                                                       ▼
          ┌─────────────────────┐                                 ┌─────────────────────┐
          │ Redis Stream & Keys │                                 │  WebSocket Clients  │
          │(market_data_stream) │                                 │      (/ws route)    │
          └─────────────────────┘                                 └─────────────────────┘
```

1. **Periodic Trigger**: Background task in `main.rs` ticks at the configured interval (`FETCH_INTERVAL_SECONDS`).
2. **Leader Evaluation**:
   - **Leader**: Calls `AppState::fetch_and_publish_market_data(true)`:
     - Aggregates market data via `MarketDataService`.
     - Caches data into Redis key `latest_market_data`.
     - Publishes to Redis Stream `market_data_stream`.
     - Broadcasts `ServerMessage::DashboardUpdate` to local WebSocket clients.
   - **Follower**: Reads `latest_market_data` directly from Redis cache and broadcasts it to its own connected clients.
3. **Client Delivery**: `Broadcaster` dispatches JSON payloads to all connected WebSocket subscribers.

---

## High Availability and Scalability

### Leader Election & Multi-Instance Scaling
- **Lock Acquisition**: Nodes attempt `SET websocket:leader <node_id> NX EX 10` in Redis.
- **Heartbeat & Renewal**: The active leader periodically executes a Lua renewal script.
- **Failover**: If the leader crashes, the lock expires within 10s. A standby follower automatically assumes leadership.
- **Efficiency**: Guarantees external API fetch calls remain constant regardless of the replica count.

---

## Technology Stack

- **Web Framework**: Axum (0.8)
- **Async Runtime**: Tokio (1.52)
- **WebSocket Protocol**: tokio-tungstenite (0.29)
- **Caching**: Multi-Tier Cache (L1 Moka + L2 Redis)
- **Serialization**: Serde & Serde JSON
- **Logging**: Tracing & Tracing Subscriber
