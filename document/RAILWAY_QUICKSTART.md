# ⚡ Railway Deployment - Quick Start

## 🚀 5 phút để deploy!

### 1️⃣ Install Railway CLI
```bash
npm i -g @railway/cli
railway login
```

### 2️⃣ Initialize Project
```bash
cd /home/thichuong/Desktop/WebReport/Web-server-Report-websocket
railway init
```
- Project name: `websocket-market-data`
- Environment: `production`

### 3️⃣ Add Redis Database
```bash
railway add redis
```
✅ `REDIS_URL` sẽ được tự động set

### 4️⃣ Set Environment Variables
```bash
# Required - API Key cho technical indicators
railway variables set TAAPI_SECRET=your_taapi_key

# Recommended - Real-time updates mỗi 5 giây
railway variables set FETCH_INTERVAL_SECONDS=5

# Optional - Fallback APIs
railway variables set CMC_API_KEY=your_cmc_key
railway variables set FINNHUB_API_KEY=your_finnhub_key

# Logging
railway variables set RUST_LOG=info
```

### 5️⃣ Configure Replicas (Multi-instance)

**Via Dashboard:**
- Vào https://railway.app
- Project → Settings → Deploy
- Set **Replicas = 3**

**Or via `railway.toml`** (đã tạo sẵn):
```toml
[deploy]
numReplicas = 3
```

### 6️⃣ Deploy!
```bash
railway up
```

### 7️⃣ Monitor Deployment
```bash
# Xem logs real-time
railway logs --follow

# Kiểm tra leader election
railway logs | grep -E "LEADER|FOLLOWER"
```

**Expected logs:**
```
[replica-1] 🎖️ LEADERSHIP ACQUIRED - Node ws-xxx is now the LEADER
[replica-1] 🎖️ [LEADER] Fetching market data from APIs...

[replica-2] 👥 [FOLLOWER] Reading market data from cache...
[replica-3] 👥 [FOLLOWER] Reading market data from cache...
```

### 8️⃣ Get WebSocket URL
```bash
railway domain
```

Example: `wss://websocket-market-data-production.up.railway.app/ws`

### 9️⃣ Test WebSocket Connection
```bash
# Get your Railway domain
RAILWAY_DOMAIN=$(railway domain)

# Connect to WebSocket
websocat wss://$RAILWAY_DOMAIN/ws
```

### 🔟 Verify Health
```bash
curl https://$(railway domain)/health
```

Expected response:
```json
{
  "status": "healthy",
  "service": "web-server-report-websocket",
  "active_connections": 0
}
```

---

## 📊 Verify Leader Election

### Check Redis Lock
```bash
railway connect redis
```

Then in Redis CLI:
```
127.0.0.1:6379> GET websocket:leader
"ws-railway-replica-id"

127.0.0.1:6379> TTL websocket:leader
(integer) 8
```

### Monitor API Calls

**With leader election (3 replicas):**
- ✅ **6 API calls/min** (chỉ leader fetch)

**Without leader election (3 replicas):**
- ❌ **18 API calls/min** (mỗi instance fetch)

**Saving: 67% reduction in API calls!** 🎉

---

## 🔧 Common Commands

```bash
# Restart all instances
railway restart

# View environment variables
railway variables

# View logs with filter
railway logs | grep ERROR
railway logs | grep LEADER

# Connect to Redis
railway connect redis

# Open dashboard
railway open

# Check service status
railway status
```

---

## 🚨 Troubleshooting

### Issue: No leader elected
```bash
# Check Redis connection
railway logs | grep "Redis"

# Verify REDIS_URL is set
railway variables get REDIS_URL

# If empty, re-add Redis
railway add redis
railway restart
```

### Issue: All instances are followers
```bash
# Check if Redis is accessible
railway connect redis
# Try: PING
# Expected: PONG

# Restart all instances
railway restart
```

### Issue: Build fails
```bash
# Check build logs
railway logs --build

# Ensure nixpacks.toml exists
ls -la nixpacks.toml

# Test local build
cargo build --release
```

---

## 📈 Scaling Guidelines

| Replicas | Leader | Followers | API Calls/min (5s interval) |
|----------|--------|-----------|------------------------------|
| 1        | 1      | 0         | 12                           |
| 3        | 1      | 2         | 12                           |
| 5        | 1      | 4         | 12                           |
| 10       | 1      | 9         | 12                           |

**Key Point:** API calls stay constant regardless of replica count! 🎯

---

## 📚 Next Steps

1. ✅ Deploy với 3 replicas
2. ✅ Monitor logs cho leadership transitions
3. ✅ Test failover (restart instances)
4. ✅ Connect WebSocket clients
5. ✅ Monitor API usage
6. ✅ Setup alerts (optional)

---

## 🔗 Useful Links

- **Railway Dashboard:** https://railway.app
- **Railway Docs:** https://docs.railway.app
- **Full Deployment Guide:** [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md)
- **Railway Discord:** https://discord.gg/railway

---

**Happy deploying! 🚀**
