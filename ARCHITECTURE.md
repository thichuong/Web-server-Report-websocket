# System Architecture

## Overview

The `web-server-report-websocket` is a Rust-based microservice responsible for real-time market data broadcasting via WebSockets. It acts as an intermediary between external cryptocurrency data providers (like Binance, CoinGecko, CoinMarketCap) and frontend clients, providing efficient data delivery with minimal external API queries.

## Core Components

The application follows a modular architecture divided into the following key layers:

1. **`api`**: Defines the HTTP and WebSocket endpoints using the `axum` framework. It manages routing (e.g., `/ws`, `/health`) and holds the shared application state (`AppState`).
2. **`services`**: Contains the core business logic of the application:
   - **Leader Election (`LeaderElectionService`)**: Utilizes Redis to coordinate multiple instances, ensuring only one designated "leader" fetches data from external APIs.
   - **Market Data (`ExternalApisIsland`)**: Interacts with third-party APIs (TAAPI, CoinMarketCap, Finnhub). It is responsible for orchestrating the fetching of market data.
   - **Broadcaster (`WebSocketServiceIsland`)**: Manages active WebSocket connections and broadcasts the formatted market data updates to all connected clients.
3. **`infrastructure`**: Provides low-level technical capabilities such as caching. It integrates the `multi-tier-cache` library (in-memory L1 cache and Redis L2 cache) to optimize data retrieval.
4. **`dto`**: Defines the Data Transfer Objects used for serialization and communication between the system's components and the varied external clients.
5. **`config`**: Handles application configuration by loading environment variables.

## Data Flow

1. **Periodic Fetching**: A background task (`spawn_market_data_fetcher` in `main.rs`) is triggered periodically based on the configured interval.
2. **Leader Check**: The instance checks if it is the current leader via the `LeaderElectionService`.
3. **Data Acquisition**:
   - **If Leader**: Fetches live data from external APIs (`ExternalApisIsland`), updates the cache, and broadcasts the data to its locally connected WebSocket clients.
   - **If Follower**: Bypasses external API calls. Instead, it reads the latest market data directly from the Redis cache (populated by the leader) and broadcasts it to its clients.
4. **Broadcasting**: The `WebSocketServiceIsland` sends the serialized data payload down the established WebSocket connections.

## High Availability and Scalability

### Leader Election Mechanism

To support horizontal scaling without exceeding external API rate limits, the service implements a robust Leader Election mechanism based on Redis distributed locks.

- **Lock Acquisition**: Instances attempt to acquire a specific lock key in Redis with a short Time-To-Live (TTL).
- **Heartbeat**: The current leader periodically extends the lock's TTL to maintain its status.
- **Failover**: If the leader crashes or becomes unresponsive, the Redis lock expires. Another follower instance will successfully acquire the lock and seamlessly take over the leader role.
- **Efficiency**: This guarantees that regardless of the number of deployed replicas, backend API calls remain constant, saving costs and preventing rate-limiting blocks.

## Technology Stack

- **Web Framework**: Axum
- **Asynchronous Runtime**: Tokio
- **Caching & Storage**: Redis (via `multi-tier-cache` & `redis` crate)
- **Serialization**: Serde
- **Configuration**: Dotenvy
