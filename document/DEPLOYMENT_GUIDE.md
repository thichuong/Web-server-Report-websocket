# 🚀 Hướng dẫn Deploy Leader Election lên Railway

## 📋 Mục lục
1. [Test Local với Multiple Instances](#test-local)
2. [Cấu hình Railway](#railway-setup)
3. [Deploy lên Railway](#deployment)
4. [Monitoring & Troubleshooting](#monitoring)
5. [Environment Variables](#environment-variables)

---

## 🧪 Test Local với Multiple Instances {#test-local}

### Bước 1: Start Redis Local

```bash
# Cài đặt Redis (nếu chưa có)
# Ubuntu/Debian:
sudo apt install redis-server

# macOS:
brew install redis

# Start Redis
redis-server

# Hoặc start như service (Ubuntu):
sudo systemctl start redis-server
```

### Bước 2: Tạo file `.env` cho testing

```bash
cd /home/thichuong/Desktop/WebReport/Web-server-Report-websocket

# Tạo .env file
cat > .env.test << 'EOF'
# Redis connection
REDIS_URL=redis://127.0.0.1:6379

# Fetch interval (5 seconds for real-time)
FETCH_INTERVAL_SECONDS=5

# API Keys (optional for testing)
TAAPI_SECRET=your_taapi_secret
CMC_API_KEY=your_coinmarketcap_key
FINNHUB_API_KEY=your_finnhub_key

# Log level
RUST_LOG=info
EOF
```

### Bước 3: Start Multiple Instances

**Terminal 1 - Instance 1 (sẽ trở thành leader):**
```bash
cd /home/thichuong/Desktop/WebReport/Web-server-Report-websocket

# Copy .env
cp .env.test .env

# Start instance 1 trên port 8081
PORT=8081 cargo run --release
```

**Terminal 2 - Instance 2 (follower):**
```bash
cd /home/thichuong/Desktop/WebReport/Web-server-Report-websocket

# Start instance 2 trên port 8082
PORT=8082 cargo run --release
```

**Terminal 3 - Instance 3 (follower):**
```bash
cd /home/thichuong/Desktop/WebReport/Web-server-Report-websocket

# Start instance 3 trên port 8083
PORT=8083 cargo run --release
```

### Bước 4: Kiểm tra logs

Bạn sẽ thấy logs như sau:

**Instance 1 (Leader):**
```
🎖️ Initializing Leader Election Service...
✅ Leader Election Service initialized!
🎖️ LEADERSHIP ACQUIRED - Node ws-xxx is now the LEADER
🎖️ [LEADER] Fetching market data from APIs...
✅ [LEADER] Market data fetched successfully from APIs
📡 [LEADER] Broadcasted to 0 WebSocket clients
```

**Instance 2 & 3 (Followers):**
```
🎖️ Initializing Leader Election Service...
✅ Leader Election Service initialized!
👥 [FOLLOWER] Reading market data from cache...
✅ [FOLLOWER] Market data loaded from cache
📡 [FOLLOWER] Broadcasted cached data to 0 WebSocket clients
```

### Bước 5: Test Failover

1. **Kill leader instance** (Ctrl+C trong Terminal 1)
2. **Đợi 5-10 giây**
3. **Kiểm tra logs** của Instance 2 hoặc 3 - một trong số chúng sẽ trở thành leader:

```
🎖️ LEADERSHIP ACQUIRED - Node ws-yyy is now the LEADER
🎖️ [LEADER] Fetching market data from APIs...
```

### Bước 6: Test WebSocket Connection

```bash
# Terminal 4 - Connect to WebSocket
websocat ws://localhost:8081/ws

# Hoặc dùng curl
curl --include \
     --no-buffer \
     --header "Connection: Upgrade" \
     --header "Upgrade: websocket" \
     --header "Sec-WebSocket-Key: SGVsbG8sIHdvcmxkIQ==" \
     --header "Sec-WebSocket-Version: 13" \
     http://localhost:8081/ws
```

Bạn sẽ nhận được market data updates mỗi 5 giây.

### Bước 7: Kiểm tra Redis

```bash
# Kết nối Redis CLI
redis-cli

# Kiểm tra leader lock
127.0.0.1:6379> GET websocket:leader
"ws-uuid-của-leader-instance"

# Kiểm tra TTL của lock
127.0.0.1:6379> TTL websocket:leader
(integer) 8

# Kiểm tra cached data
127.0.0.1:6379> GET latest_market_data
# Sẽ hiển thị JSON data

# Thoát
127.0.0.1:6379> exit
```

---

## ☁️ Cấu hình Railway {#railway-setup}

### Bước 1: Tạo Railway Project

1. **Đăng nhập Railway:**
   ```bash
   # Cài Railway CLI (nếu chưa có)
   npm i -g @railway/cli

   # Login
   railway login
   ```

2. **Tạo project mới:**
   ```bash
   cd /home/thichuong/Desktop/WebReport/Web-server-Report-websocket

   # Initialize Railway project
   railway init
   ```

3. **Chọn options:**
   - Project name: `websocket-market-data`
   - Environment: `production`

### Bước 2: Add Redis Database

**Option A: Via Railway Dashboard (Khuyến nghị)**
1. Vào https://railway.app
2. Chọn project `websocket-market-data`
3. Click `+ New` → `Database` → `Add Redis`
4. Redis instance sẽ tự động được tạo
5. Railway tự động tạo biến `REDIS_URL`

**Option B: Via CLI**
```bash
railway add redis
```

### Bước 3: Configure Environment Variables

**Via Railway Dashboard:**
1. Vào project → `Variables` tab
2. Add các biến sau:

```bash
# Required
REDIS_URL=redis://default:password@host:port (auto-populated by Railway)

# Fetch interval
FETCH_INTERVAL_SECONDS=5

# API Keys
TAAPI_SECRET=your_taapi_secret_here
CMC_API_KEY=your_coinmarketcap_key_here
FINNHUB_API_KEY=your_finnhub_key_here

# Logging
RUST_LOG=info

# Host/Port (Railway tự động set)
HOST=0.0.0.0
PORT=8080
```

**Via CLI:**
```bash
railway variables set FETCH_INTERVAL_SECONDS=5
railway variables set TAAPI_SECRET=your_key
railway variables set CMC_API_KEY=your_key
railway variables set FINNHUB_API_KEY=your_key
railway variables set RUST_LOG=info
```

### Bước 4: Configure Scaling (Multiple Instances)

**Via Railway Dashboard:**
1. Project → `Settings` tab
2. Scroll to `Deploy` section
3. Set `Replicas` = **3** (hoặc số instance bạn muốn)

**Via `railway.toml` (Khuyến nghị):**

Tạo file `railway.toml`:

```bash
cd /home/thichuong/Desktop/WebReport/Web-server-Report-websocket

cat > railway.toml << 'EOF'
[build]
builder = "nixpacks"
buildCommand = "cargo build --release"

[deploy]
# Number of instances (replicas)
numReplicas = 3

# Start command
startCommand = "./target/release/web-server-report-websocket"

# Health check
healthcheckPath = "/health"
healthcheckTimeout = 30

# Restart policy
restartPolicyType = "on-failure"
restartPolicyMaxRetries = 10

[service]
# Railway will assign dynamic port
internalPort = 8080
EOF
```

### Bước 5: Configure Nixpacks (Rust Build)

Tạo file `nixpacks.toml`:

```bash
cat > nixpacks.toml << 'EOF'
[phases.setup]
nixPkgs = ["rust", "openssl", "pkg-config", "protobuf"]

[phases.build]
cmds = ["cargo build --release"]

[start]
cmd = "./target/release/web-server-report-websocket"
EOF
```

### Bước 6: Configure `.railwayignore`

```bash
cat > .railwayignore << 'EOF'
# Development files
.env
.env.test
.env.local

# Build artifacts
target/debug/
*.pdb

# IDE
.vscode/
.idea/
*.swp
*.swo

# Test files
tests/
benches/

# Documentation
docs/
*.md
!README.md
EOF
```

---

## 🚢 Deploy lên Railway {#deployment}

### Method 1: Via CLI (Khuyến nghị)

```bash
cd /home/thichuong/Desktop/WebReport/Web-server-Report-websocket

# Deploy
railway up

# Theo dõi deployment logs
railway logs
```

### Method 2: Via Git Push

```bash
# Tạo Git repo (nếu chưa có)
git init
git add .
git commit -m "Add leader election for multi-instance deployment"

# Link với Railway
railway link

# Push to deploy
git push railway main
```

### Method 3: Via GitHub Integration

1. Push code lên GitHub:
   ```bash
   git remote add origin https://github.com/your-username/websocket-service.git
   git push -u origin main
   ```

2. Railway Dashboard:
   - Project → `Settings` → `GitHub Repo`
   - Connect repository
   - Auto-deploy on push enabled by default

---

## 📊 Monitoring & Troubleshooting {#monitoring}

### View Logs in Real-time

```bash
# All instances logs
railway logs

# Follow logs (real-time)
railway logs --follow

# Filter by keyword
railway logs | grep "LEADER"
railway logs | grep "FOLLOWER"
```

### Check Health Endpoint

```bash
# Get your Railway URL
railway domain

# Example: https://websocket-market-data-production.up.railway.app

# Check health
curl https://your-app.up.railway.app/health

# Expected response:
{
  "status": "healthy",
  "service": "web-server-report-websocket",
  "active_connections": 0
}
```

### Monitor Leader Election

**Check Redis via Railway CLI:**
```bash
# Connect to Railway Redis
railway connect redis

# Once connected:
127.0.0.1:6379> GET websocket:leader
"ws-railway-replica-id"

127.0.0.1:6379> TTL websocket:leader
(integer) 7
```

**Via logs - identify leader:**
```bash
# Leader logs
railway logs | grep "LEADER"

# Example output:
[replica-1] 🎖️ LEADERSHIP ACQUIRED - Node ws-abc123 is now the LEADER
[replica-1] 🎖️ [LEADER] Fetching market data from APIs...
```

```bash
# Follower logs
railway logs | grep "FOLLOWER"

# Example output:
[replica-2] 👥 [FOLLOWER] Reading market data from cache...
[replica-3] 👥 [FOLLOWER] Reading market data from cache...
```

### Verify Failover

**Test 1: Restart leader instance**
```bash
# Via Railway dashboard: Service → ... menu → Restart
# Or via CLI:
railway restart

# Monitor logs for leadership transition:
railway logs --follow | grep -E "LEADER|FOLLOWER"

# Expected:
[replica-1] 🔄 LEADERSHIP LOST - Node ws-abc123 is now a FOLLOWER
[replica-2] 🎖️ LEADERSHIP ACQUIRED - Node ws-def456 is now the LEADER
```

### Common Issues & Solutions

#### Issue 1: All instances are followers
**Symptom:**
```
👥 [FOLLOWER] Reading market data from cache...
⚠️ [FOLLOWER] No cached data available yet
```

**Solution:**
```bash
# Check Redis connectivity
railway logs | grep "Redis"

# Verify REDIS_URL is set
railway variables get REDIS_URL

# If empty, re-add Redis database
railway add redis
```

#### Issue 2: Multiple leaders (split brain)
**Symptom:**
```
[replica-1] 🎖️ [LEADER] Fetching...
[replica-2] 🎖️ [LEADER] Fetching...
```

**Solution:**
```bash
# This shouldn't happen with proper Redis setup
# Check Redis connection:
railway connect redis
127.0.0.1:6379> GET websocket:leader

# If issue persists, restart all instances:
railway restart
```

#### Issue 3: Build fails
**Symptom:**
```
error: could not compile `web-server-report-websocket`
```

**Solution:**
```bash
# Check Rust version in nixpacks.toml
# Ensure all dependencies in Cargo.toml are accessible

# Test build locally:
cargo build --release

# If local build works, check Railway logs:
railway logs --build
```

#### Issue 4: WebSocket connections drop
**Symptom:**
Clients disconnect after 30-60 seconds

**Solution:**
```bash
# Railway has connection timeout limits
# Add keepalive in WebSocket handler (already implemented)

# Increase healthcheck timeout in railway.toml:
healthcheckTimeout = 60
```

---

## 🔧 Environment Variables Reference {#environment-variables}

### Required Variables

| Variable | Description | Example | Auto-set by Railway |
|----------|-------------|---------|---------------------|
| `REDIS_URL` | Redis connection string | `redis://default:pass@host:6379` | ✅ Yes (when Redis added) |
| `PORT` | HTTP server port | `8080` | ✅ Yes |

### Optional Variables

| Variable | Description | Default | Recommended |
|----------|-------------|---------|-------------|
| `FETCH_INTERVAL_SECONDS` | API fetch interval | `10` | `5` |
| `HOST` | Server bind address | `0.0.0.0` | `0.0.0.0` |
| `RUST_LOG` | Log level | `info` | `info` or `debug` |
| `TAAPI_SECRET` | TAAPI.io API key | - | Required for RSI data |
| `CMC_API_KEY` | CoinMarketCap key | - | Optional (fallback) |
| `FINNHUB_API_KEY` | Finnhub API key | - | Required for US stocks |

### Railway-specific Variables (Auto-set)

| Variable | Description | Example |
|----------|-------------|---------|
| `RAILWAY_ENVIRONMENT` | Environment name | `production` |
| `RAILWAY_PROJECT_ID` | Project ID | `abc-123` |
| `RAILWAY_REPLICA_ID` | Instance replica ID | `replica-1` |
| `RAILWAY_INSTANCE_ID` | Unique instance ID | `instance-xyz` |

**Note:** The leader election service automatically uses `RAILWAY_REPLICA_ID` or `RAILWAY_INSTANCE_ID` for unique node identification.

---

## 📈 Performance Tuning

### Recommended Settings

**For 3 replicas (1 leader + 2 followers):**
- `FETCH_INTERVAL_SECONDS=5` (real-time updates)
- Leader fetches API every 5s
- Followers read from cache every 5s
- **Total API calls:** 6/min (vs 18/min without leader election)

**For high traffic (100+ WebSocket clients):**
- Consider increasing replicas to 5-7
- Still only 1 leader fetches APIs
- More followers = better WebSocket distribution

**For low API rate limits:**
- Increase `FETCH_INTERVAL_SECONDS=10` or `15`
- Reduces API calls proportionally
- Clients still get updates, just less frequently

---

## 🎯 Next Steps

1. **✅ Deploy to Railway** với 3 replicas
2. **✅ Monitor logs** để verify leader election
3. **✅ Test failover** bằng cách restart instances
4. **✅ Connect WebSocket clients** và verify real-time updates
5. **✅ Monitor API usage** để ensure rate limits không bị vượt
6. **✅ Setup alerts** (optional) via Railway webhooks

---

## 📞 Support

- **Railway Docs:** https://docs.railway.app
- **Railway Discord:** https://discord.gg/railway
- **Project Issues:** Check Railway project logs và Redis connectivity

---

**Chúc bạn deploy thành công! 🚀**
