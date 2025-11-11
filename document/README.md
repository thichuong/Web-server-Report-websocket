# 📚 Documentation Index

Thư mục này chứa tất cả tài liệu hướng dẫn cho WebSocket Market Data Service với Leader Election.

---

## 📖 Documents

### 1. [RAILWAY_QUICKSTART.md](./RAILWAY_QUICKSTART.md) ⚡
**Quick Start Guide - 5 phút deploy lên Railway**

- Hướng dẫn nhanh nhất để deploy
- Commands step-by-step
- Verification và monitoring cơ bản
- Perfect cho: Người muốn deploy ngay lập tức

**Đọc file này nếu:** Bạn muốn deploy ngay trong 5 phút

---

### 2. [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md) 📖
**Complete Deployment Guide - Hướng dẫn đầy đủ chi tiết**

- Test local với multiple instances
- Railway configuration chi tiết
- Deployment methods (CLI, Git, GitHub)
- Monitoring & troubleshooting
- Environment variables reference
- Performance tuning
- Common issues và solutions

**Đọc file này nếu:** Bạn muốn hiểu sâu về deployment process

---

### 3. [LEADER_ELECTION_SUMMARY.md](./LEADER_ELECTION_SUMMARY.md) 🎖️
**Technical Implementation Summary**

- Architecture overview
- Files created/modified
- Key components (Leader Election Service)
- Configuration details
- Performance metrics
- Data flow timeline
- Testing & verification
- Design decisions explained

**Đọc file này nếu:** Bạn muốn hiểu technical implementation

---

## 🚀 Suggested Reading Order

### For Deployment:
1. **[RAILWAY_QUICKSTART.md](./RAILWAY_QUICKSTART.md)** - Deploy ngay
2. **[DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md)** - Nếu gặp vấn đề

### For Understanding:
1. **[LEADER_ELECTION_SUMMARY.md](./LEADER_ELECTION_SUMMARY.md)** - Technical details
2. **[DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md)** - Deployment context

---

## 📁 Project Structure

```
Web-server-Report-websocket/
├── document/                           # Documentation (this folder)
│   ├── README.md                       # This file
│   ├── RAILWAY_QUICKSTART.md           # Quick start guide
│   ├── DEPLOYMENT_GUIDE.md             # Full deployment guide
│   └── LEADER_ELECTION_SUMMARY.md      # Technical summary
│
├── src/                                # Source code
│   └── service_islands/
│       └── layer1_infrastructure/
│           └── distributed_coordination/
│               ├── mod.rs
│               └── leader_election.rs  # Leader election implementation
│
├── .env.railway.example                # Railway env vars template
├── railway.toml                        # Railway deployment config
├── nixpacks.toml                       # Rust build config
├── .railwayignore                      # Files to ignore in Railway
├── Cargo.toml                          # Rust dependencies
└── README.md                           # Project README
```

---

## 🔗 External Resources

- **Railway Platform:** https://railway.app
- **Railway Docs:** https://docs.railway.app
- **Redis Documentation:** https://redis.io/docs
- **Rust Book:** https://doc.rust-lang.org/book/

---

## 📝 Quick Reference

### Environment Variables
```bash
REDIS_URL=redis://...           # Auto-set by Railway
FETCH_INTERVAL_SECONDS=5        # API fetch interval
TAAPI_SECRET=your_key           # Required
CMC_API_KEY=your_key            # Optional
FINNHUB_API_KEY=your_key        # Optional
```

### Railway Commands
```bash
railway init                    # Initialize project
railway add redis              # Add Redis database
railway variables set KEY=VAL  # Set environment variable
railway up                     # Deploy
railway logs --follow          # View logs
railway restart                # Restart service
```

### Verify Deployment
```bash
# Check health
curl https://your-app.railway.app/health

# View leader election
railway logs | grep -E "LEADER|FOLLOWER"

# Connect to Redis
railway connect redis
```

---

## ✅ Checklist

### Before Deployment
- [ ] Đọc RAILWAY_QUICKSTART.md
- [ ] Railway CLI installed
- [ ] API keys prepared (TAAPI_SECRET)
- [ ] Code compiled (`cargo build --release`)

### During Deployment
- [ ] Railway project created
- [ ] Redis database added
- [ ] Environment variables set
- [ ] Replicas configured (3+)
- [ ] Deployment successful

### After Deployment
- [ ] Health check passes
- [ ] Leader elected (check logs)
- [ ] WebSocket connections work
- [ ] Failover tested
- [ ] API call rate verified

---

## 🆘 Need Help?

1. **Quick issues:** Check [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md) → Troubleshooting section
2. **Railway specific:** Railway Discord - https://discord.gg/railway
3. **Redis issues:** Check Redis connection in logs
4. **Build issues:** Check [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md) → Common Issues

---

**Last Updated:** 2025-11-11
**Version:** 1.0.0
