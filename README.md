# Web Server Report - WebSocket Service

WebSocket microservice cho hệ thống Web Report. Service này chịu trách nhiệm:

- 🔌 WebSocket connections và real-time broadcasting
- 🌐 External API calls (Binance, CoinGecko, CoinMarketCap, etc.)
- 📡 Publishing market data to Redis Streams
- 💾 Populating cache for main service

## Architecture

```
WebSocket Service (Port 8081)
    ↓
External APIs → Cache → Redis Streams → WebSocket Broadcast
```

## Prerequisites

- Rust 1.70+
- Redis server
- API Keys:
  - TAAPI_SECRET (required)
  - CMC_API_KEY (optional)
  - FINNHUB_API_KEY (optional)

## Dependencies

This service requires the `multi-tier-cache` library. Ensure it's available:

```bash
# If multi-tier-cache is in parent directory:
cd ..
git clone <multi-tier-cache-repo>

# Or ensure it's at ../multi-tier-cache relative to this project
```

## Quick Start

1. **Copy environment template:**
   ```bash
   cp .env.example .env
   ```

2. **Edit `.env` with your API keys:**
   ```bash
   TAAPI_SECRET=your_taapi_secret
   CMC_API_KEY=your_cmc_key
   FINNHUB_API_KEY=your_finnhub_key
   ```

3. **Start Redis:**
   ```bash
   redis-server
   ```

4. **Run service:**
   ```bash
   cargo run --release
   ```

## Configuration

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `HOST` | Server host | `0.0.0.0` | No |
| `PORT` | Server port | `8081` | No |
| `REDIS_URL` | Redis connection | `redis://localhost:6379` | Yes |
| `FETCH_INTERVAL_SECONDS` | Data fetch interval | `10` | No |
| `TAAPI_SECRET` | TAAPI.io API key | - | Yes |
| `CMC_API_KEY` | CoinMarketCap key | - | No |
| `FINNHUB_API_KEY` | Finnhub key | - | No |

## Endpoints

- **WebSocket:** `ws://localhost:8081/ws`
- **Health Check:** `http://localhost:8081/health`

## Development

```bash
# Check compilation
cargo check

# Run with logs
RUST_LOG=debug cargo run

# Build release
cargo build --release
```

## Docker

```bash
# Build image
docker build -t web-server-report-websocket .

# Run container
docker run -p 8081:8081 \
  -e REDIS_URL=redis://host.docker.internal:6379 \
  -e TAAPI_SECRET=your_key \
  web-server-report-websocket
```

## Integration with Main Service

The main Web-server-Report service reads data from:
1. **Cache** (populated by this service)
2. **Redis Streams** (`market_data_stream`)

This service publishes data every 10 seconds (configurable).

## License

Apache-2.0

## Authors

thichuong
