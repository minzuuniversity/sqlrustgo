# SQLRustGo 168h SOAK Test Plan

## Overview

Long-running stability test (168 hours / 7 days) for SQLRustGo v3.11.0 using TPC-H SF=0.1 dataset.

## Test Configuration

### Environment
- **OS**: macOS (Darwin 25.5.0, arm64)
- **Rust Version**: 2024 edition with Tokio async runtime
- **Build Target**: Release (`--release`)

### Binary Configuration
- **Server Binary**: `target/release/sqlrustgo-mysql-server`
- **Test Client**: `target/release/examples/hybrid_soak`
- **Storage**: Binary format (no WAL for performance)
- **Server Threads**: 8
- **Test Port**: 3396

### Dataset
- **TPC-H Scale Factor**: 0.1
- **Total Rows**: ~866,000
- **Storage Size**: ~160MB (binary format)
- **Tables**: nation (25), region (5), supplier (1K), part (20K), partsupp (80K), customer (15K), orders (150K), lineitem (600K)

## Test Parameters

| Parameter | Value |
|-----------|-------|
| Duration | 604,800 seconds (168 hours) |
| Target QPS | 50 per thread (200 total) |
| Threads | 4 |
| OLTP Ratio | 0.32 (32% OLTP, 68% OLAP) |
| Report Interval | 300 seconds |
| Log Level | error (to prevent disk exhaustion) |

## Prerequisites

### 1. Build Binaries

```bash
# Clone and build
cargo build --release --package sqlrustgo-mysql-server
cargo build --release --package sqlrustgo-mysql-client --example hybrid_soak

# Verify binaries exist
ls -la target/release/sqlrustgo-mysql-server
ls -la target/release/examples/hybrid_soak
```

### 2. Prepare TPC-H Data

```bash
# Create data directory
mkdir -p /tmp/sqlrustgo-v311-soak

# Load TPC-H SF=0.1 data using bulk loader
python3 /tmp/bulk_load_default.py --data-dir /tmp/sqlrustgo-v311-soak --tpch-sf 0.1
```

Or manually load using mysql client:

```bash
mysql -h 127.0.0.1 -P 3396 -u root default
```

## Test Execution

### Start Server

```bash
nohup ./target/release/sqlrustgo-mysql-server serve \
  --port 3396 \
  --data-dir /tmp/sqlrustgo-v311-soak \
  --storage binary \
  --server-threads 8 \
  --log-level error \
  > /tmp/soak_server.log 2>&1 &

echo "Server PID: $!"
```

### Start Client

```bash
nohup ./target/release/examples/hybrid_soak \
  --host 127.0.0.1 \
  --port 3396 \
  --threads 4 \
  --duration 604800 \
  --target-qps 50 \
  --report-interval 300 \
  > /tmp/soak_client.log 2>&1 &

echo "Client PID: $!"
```

### Monitor Test

```bash
# Watch client output
tail -f /tmp/soak_client.log

# Watch server output
tail -f /tmp/soak_server.log

# Check process status
ps aux | grep sqlrustgo
ps aux | grep hybrid_soak

# Check connections
lsof -i :3396

# Check resource usage
ps -p <SERVER_PID> -o pid,rss,vsz,%cpu
```

### Status Report Script

```bash
#!/bin/bash
echo "=============================================="
echo "       168h SOAK Test Report"
echo "=============================================="
echo "Time: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

SERVER_PID=$(pgrep -f "sqlrustgo-mysql-server" | head -1)
CLIENT_PID=$(pgrep -f "hybrid_soak" | head -1)

# Get elapsed time
LAST=$(grep "^\[" /tmp/soak_client.log 2>/dev/null | tail -1)
ELAPSED=$(echo "$LAST" | grep -oE "^\[ *[0-9]+" | grep -oE "[0-9]+" || echo 0)
HOURS=$(awk "BEGIN {printf \"%.1f\", $ELAPSED / 3600}")

# Parse metrics
QPS=$(echo "$LAST" | grep -oE "qps= *[0-9]+" | grep -oE "[0-9]+" || echo 0)
TOT_OPS=$(echo "$LAST" | grep -oE "tot_ops=[0-9]+" | cut -d= -f2 || echo 0)
OLTP=$(echo "$LAST" | grep -oE "oltp=[0-9]+/[0-9]+" | cut -d= -f2 || echo "0/0")
OLAP=$(echo "$LAST" | grep -oE "olap=[0-9]+/[0-9]+" | cut -d= -f2 || echo "0/0")

# Server
SERVER_RSS=$(ps -p $SERVER_PID -o rss= 2>/dev/null | tr -d ' ' || echo 0)
SERVER_RSS_GB=$(awk "BEGIN {printf \"%.2f\", $SERVER_RSS / 1024 / 1024}")
CONNS=$(lsof -i :3396 2>/dev/null | grep ESTABLISHED | wc -l | tr -d ' ' || echo 0)

# Disk
DISK_FREE=$(df -h /tmp | tail -1 | awk '{print $4}')

echo "【运行时间】 ${HOURS}h (${ELAPSED}s)"
echo ""
echo "【进程】"
echo "  Server PID=$SERVER_PID RSS=${SERVER_RSS_GB}GB"
echo "  Client PID=$CLIENT_PID"
echo ""
echo "【连接数】$CONNS"
echo ""
echo "【QPS / TPS】"
echo "  QPS: $QPS/sec"
echo "  Total Ops: $TOT_OPS"
echo ""
echo "【OLTP】$OLTP"
echo "【OLAP】$OLAP"
echo ""
echo "【磁盘】Free: $DISK_FREE"
echo ""
echo "【Client 最后报告】"
echo "$LAST"
echo "=============================================="
```

## Success Criteria

| Metric | Target | Threshold |
|--------|--------|-----------|
| Uptime | 168h | > 95% (160h minimum) |
| Error Rate | < 1% | < 5% acceptable |
| Memory Stability | RSS stable | No unbounded growth |
| QPS | > 50/sec | > 25/sec minimum |
| No Deadlocks | 0 | Any occurrence is failure |

## Known Issues

### Binary Storage Limitations

1. **No WAL**: Binary storage mode does not support Write-Ahead Logging
   - Triggers are silently skipped
   - No durability protection on crash
   - Suitable for testing only, not production

2. **Thread Safety**: BinaryTableStorage uses `parking_lot::RwLock` internally
   - Multiple server threads can safely access tables
   - Potential for lock contention at high concurrency

### Connection Issues

If baseline connection fails with "Resource temporarily unavailable":
- This indicates socket buffer exhaustion
- Reduce `--target-qps` or increase OS socket buffer size
- Check `ulimit -n` for file descriptor limits

## Troubleshooting

### Server Won't Start
```bash
# Check port availability
lsof -i :3396

# Check data directory permissions
ls -la /tmp/sqlrustgo-v311-soak/

# View server logs
cat /tmp/soak_server.log
```

### Client Fails to Connect
```bash
# Verify server is listening
lsof -i :3396 | grep LISTEN

# Test connection manually
mysql -h 127.0.0.1 -P 3396 -u root -e "SELECT 1"

# Check client logs
cat /tmp/soak_client.log
```

### High Error Rate
```bash
# Check error types in server log
grep "ERROR" /tmp/soak_server.log | head -20

# Check for deadlocks
grep -i "deadlock\|panic" /tmp/soak_server.log
```

### Memory Growth
```bash
# Monitor RSS over time
while true; do
  ps -p <PID> -o rss=
  sleep 60
done

# Check for memory leaks with Instruments
instruments -t Leaks ./target/release/sqlrustgo-mysql-server
```

## Test Artifacts

| Artifact | Location | Purpose |
|----------|----------|---------|
| Server Log | `/tmp/soak_server.log` | Server execution trace |
| Client Log | `/tmp/soak_client.log` | Client metrics and errors |
| Data Directory | `/tmp/sqlrustgo-v311-soak/` | Binary table files |

## Related Issues

- Issue #3500: Binary storage thread safety investigation
- Issue #3557: hybrid_soak baseline connection leak (fixed)
- Issue #3864: PR fixing hybrid_soak connection leak

## Contact

For questions or issues, contact the SQLRustGo team via:
- Gitea: http://192.168.0.250:3000/openclaw/sqlrustgo
- Email: sqlrustgo@example.com
