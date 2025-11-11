# Web Server Report - WebSocket Service

WebSocket microservice cho hệ thống Web Report với **Leader Election** cho multi-instance deployment.

## ✨ Features

- 🔌 WebSocket connections và real-time broadcasting
- 🌐 External API calls (Binance, CoinGecko, CoinMarketCap, etc.)
- 📡 Publishing market data to Redis Streams
- 💾 Populating cache for main service
- 🎖️ **Leader Election** - Only 1 instance fetches APIs (giảm 67% API calls)
- 🔄 **Auto Failover** - Automatic leadership transfer khi leader crashes
- ☁️ **Railway Ready** - Production deployment configuration included

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

This service publishes data every 5-10 seconds (configurable).

---

## 🎖️ Leader Election (Multi-Instance Deployment)

### Overview

Khi deploy với multiple instances (replicas), chỉ **1 instance (leader)** sẽ fetch APIs, các instances còn lại (followers) đọc từ Redis cache.

**Benefits:**
- ✅ Giảm 67% API calls (3 instances: 18 → 6 calls/min)
- ✅ Tránh rate limiting từ external APIs
- ✅ Auto failover 5-10 giây khi leader crashes
- ✅ Horizontal scaling không tăng API usage

### Architecture with Leader Election

```
┌─────────────────────────────────────────────────────┐
│            Railway Platform (3 replicas)            │
├─────────────────────────────────────────────────────┤
│  Instance 1        Instance 2        Instance 3     │
│  [LEADER] ✅       [FOLLOWER]        [FOLLOWER]     │
│  ├─ Fetch API      ├─ Read Cache     ├─ Read Cache │
│  ├─ Store Redis    ├─ Broadcast      ├─ Broadcast  │
│  └─ Broadcast      └─ ...            └─ ...        │
└─────────────────────────────────────────────────────┘
                         │
                  ┌──────▼──────┐
                  │    Redis    │
                  │ ├─ Lock     │ ← Leader election
                  │ └─ Cache    │ ← Market data
                  └─────────────┘
```

### Quick Deploy to Railway

```bash
# 1. Install Railway CLI
npm i -g @railway/cli && railway login

# 2. Initialize project
railway init

# 3. Add Redis database
railway add redis

# 4. Set environment variables
railway variables set TAAPI_SECRET=your_key
railway variables set FETCH_INTERVAL_SECONDS=5

# 5. Deploy (3 replicas configured in railway.toml)
railway up

# 6. Monitor logs
railway logs --follow | grep -E "LEADER|FOLLOWER"
```

**Expected logs:**
```
[replica-1] 🎖️ LEADERSHIP ACQUIRED - Node ws-xxx is now the LEADER
[replica-1] 🎖️ [LEADER] Fetching market data from APIs...
[replica-2] 👥 [FOLLOWER] Reading market data from cache...
[replica-3] 👥 [FOLLOWER] Reading market data from cache...
```

---

## 📚 Documentation

Toàn bộ documentation nằm trong thư mục [`document/`](./document/):

### Quick Links

- **[⚡ Quick Start (5 phút)](./document/RAILWAY_QUICKSTART.md)** - Deploy lên Railway ngay lập tức
- **[📖 Full Deployment Guide](./document/DEPLOYMENT_GUIDE.md)** - Hướng dẫn đầy đủ chi tiết
- **[🎖️ Leader Election Technical Summary](./document/LEADER_ELECTION_SUMMARY.md)** - Technical implementation details
- **[📁 Documentation Index](./document/README.md)** - Danh sách toàn bộ tài liệu

### When to Read What?

| Situation | Read This |
|-----------|-----------|
| 🚀 Muốn deploy ngay | [RAILWAY_QUICKSTART.md](./document/RAILWAY_QUICKSTART.md) |
| 🐛 Gặp lỗi khi deploy | [DEPLOYMENT_GUIDE.md](./document/DEPLOYMENT_GUIDE.md) → Troubleshooting |
| 🧑‍💻 Muốn hiểu code | [LEADER_ELECTION_SUMMARY.md](./document/LEADER_ELECTION_SUMMARY.md) |
| 📋 Cần reference nhanh | [document/README.md](./document/README.md) |

---

## 🧪 Testing Multi-Instance Locally

```bash
# Terminal 1: Redis
redis-server

# Terminal 2-4: Start 3 instances
PORT=8081 cargo run --release  # Will become leader
PORT=8082 cargo run --release  # Follower
PORT=8083 cargo run --release  # Follower

# Verify in Redis
redis-cli
127.0.0.1:6379> GET websocket:leader
127.0.0.1:6379> TTL websocket:leader
```

**Test Failover:**
1. Kill leader instance (Ctrl+C in Terminal 2)
2. Wait 5-10 seconds
3. Check logs - một follower sẽ trở thành leader

---

## License

Apache-2.0

## Authors

thichuong
