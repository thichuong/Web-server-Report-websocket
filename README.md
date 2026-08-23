# Web Server Report - WebSocket Service

WebSocket microservice cho hệ thống Web Report với **Leader Election** cho multi-instance deployment và tổng hợp dữ liệu thị trường tiền điện tử thời gian thực.

## ✨ Features

- 🔌 **Real-time WebSocket Streaming**: Phát trực tiếp dữ liệu thị trường tới client với độ trễ thấp qua Axum WebSocket.
- ⚡ **In-Memory Price Feeds**: Tích hợp trực tiếp Binance Combined WebSocket stream cho 7 đồng coin (BTC, ETH, SOL, XRP, ADA, LINK, BNB) với fallback Binance US & REST.
- 🌐 **External Market Indicators**: Tự động tổng hợp dữ liệu toàn cầu từ CoinGecko (fallback CoinMarketCap), Fear & Greed Index (Alternative.me), và RSI-14 (TAAPI.io).
- 💾 **Multi-Tier Caching**: Tích hợp L1 Moka in-memory cache và L2 Redis cache giúp tối ưu tốc độ và giảm tối đa số lần gọi API.
- 📡 **Redis Streams Integration**: Đẩy dữ liệu thị trường mới nhất vào Redis Stream `market_data_stream` và key `latest_market_data`.
- 🎖️ **Leader Election**: Chỉ 1 instance (Leader) gọi external APIs, các followers đọc từ Redis cache (giảm 67%+ API quota).
- 🔄 **Auto Failover**: Tự động chuyển giao quyền Leader trong 5-10 giây khi Leader gặp sự cố.
- ☁️ **Production Ready**: Cấu hình sẵn sàng cho Railway, Docker, Kubernetes.

---

## 🏛️ Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    External Cryptocurrency Sources                      │
│   Binance WS (Primary) ──► Binance REST ──► CoinGecko ──► TAAPI / FNG   │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
                      ┌─────────────────────────────┐
                      │      MarketDataService      │
                      │  (Aggregator & In-Memory)   │
                      └──────────────┬──────────────┘
                                     │
                     ┌───────────────┴───────────────┐
                     │                               │
                     ▼                               ▼
          ┌─────────────────────┐         ┌─────────────────────┐
          │  Multi-Tier Cache   │         │     Broadcaster     │
          │ (L1 Moka + L2 Redis)│         │ (broadcast channel) │
          └──────────┬──────────┘         └──────────┬──────────┘
                     │                               │
                     ▼                               ▼
          ┌─────────────────────┐         ┌─────────────────────┐
          │ Redis Stream & Keys │         │  WebSocket Clients  │
          │(market_data_stream) │         │     (ws://.../ws)   │
          └─────────────────────┘         └─────────────────────┘
```

Xem chi tiết kiến trúc hệ thống tại [ARCHITECTURE.md](./ARCHITECTURE.md).

---

## 📋 Prerequisites

- **Rust**: 1.80+ (Edition 2024 / 2021)
- **Redis Server**: 6.0+
- **API Keys** (Tùy chọn, có fallback mặc định nếu chưa cấu hình):
  - `TAAPI_SECRET` (Tùy chọn - mặc định trả về RSI trung tính `50.0` nếu chưa cấu hình)
  - `CMC_API_KEY` (Tùy chọn - CoinMarketCap fallback)

---

## 🚀 Quick Start

1. **Copy file môi trường:**
   ```bash
   cp .env.example .env
   ```

2. **Cấu hình `.env`:**
   ```bash
   HOST=0.0.0.0
   PORT=8081
   REDIS_URL=redis://127.0.0.1:6379
   FETCH_INTERVAL_SECONDS=5
   TAAPI_SECRET=your_taapi_secret_here
   CMC_API_KEY=your_cmc_key_here
   ```

3. **Khởi động Redis:**
   ```bash
   redis-server
   ```

4. **Chạy dịch vụ:**
   ```bash
   cargo run --release
   ```

---

## ⚙️ Configuration Reference

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `HOST` | Host lắng nghe của server | `0.0.0.0` | No |
| `PORT` | Cổng dịch vụ HTTP & WebSocket | `8081` | No |
| `REDIS_URL` | Địa chỉ kết nối Redis | `redis://127.0.0.1:6379` | Yes |
| `FETCH_INTERVAL_SECONDS` | Tần suất fetch dữ liệu (giây) | `5` | No |
| `TAAPI_SECRET` | Secret key của TAAPI.io | `default_secret` | No |
| `CMC_API_KEY` | API Key của CoinMarketCap | None | No |

---

## 🌐 Endpoints

- **WebSocket Stream:** `ws://localhost:8081/ws`
- **Health Check:** `http://localhost:8081/health`

---

## 🧪 Testing & Code Quality

```bash
# Chạy toàn bộ Unit & Integration tests
cargo test

# Kiểm tra quy tắc Strictly No Unwrap & Code Quality
cargo clippy -- -D clippy::unwrap_used

# Kiểm tra format mã nguồn
cargo fmt --check
```

---

## 🐳 Docker Deployment

```bash
# Build Docker image
docker build -t web-server-report-websocket .

# Chạy container
docker run -p 8081:8081 \
  -e REDIS_URL=redis://host.docker.internal:6379 \
  -e TAAPI_SECRET=your_key \
  web-server-report-websocket
```

---

## 🎖️ Leader Election (Multi-Instance Deployment)

Khi triển khai nhiều bản sao (replicas), chỉ **1 instance (Leader)** chịu trách nhiệm gọi API bên ngoài. Các instances còn lại (**Followers**) sẽ đọc dữ liệu đã cache từ Redis.

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

**Logs hiển thị khi hoạt động:**
```text
[instance-1] 🎖️ LEADERSHIP ACQUIRED - Node ws-xxx is now the LEADER
[instance-1] 🎖️ [LEADER] Fetching market data from APIs...
[instance-1] 📡 [LEADER] Broadcasted to 2 WebSocket clients
[instance-2] 👥 [FOLLOWER] Reading market data from cache...
[instance-2] 📡 [FOLLOWER] Broadcasted cached data to 1 WebSocket clients
```

---

## 📚 Documentation Index

- **[🏛️ Architecture Overview](./ARCHITECTURE.md)**: Chi tiết cấu trúc module và luồng dữ liệu.
- **[⚡ Railway Quickstart](./document/RAILWAY_QUICKSTART.md)**: Hướng dẫn deploy lên Railway trong 5 phút.
- **[📖 Deployment Guide](./document/DEPLOYMENT_GUIDE.md)**: Hướng dẫn triển khai chi tiết cho production.
- **[🎖️ Leader Election Summary](./document/LEADER_ELECTION_SUMMARY.md)**: Chi tiết cơ chế bầu cử và distributed lock.

---

## License

Apache-2.0
