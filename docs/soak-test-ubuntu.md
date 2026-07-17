# SQLRustGo SOAK Test - Ubuntu Linux x86_64

## HP Z440 Ubuntu Setup Guide

HP Z440 Workstation specifications:
- CPU: Intel Xeon (x86_64)
- OS: Ubuntu Linux
- Memory: Configure based on availability

## Prerequisites

### 1. Install Rust

```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### 2. Clone Repository

```bash
# Clone the repository
git clone http://192.168.0.250:3000/openclaw/sqlrustgo.git
cd sqlrustgo

# Or if already cloned, update to latest
git fetch origin
git checkout develop/v3.11.0
git pull origin develop/v3.11.0
```

### 3. Build Binaries

```bash
# Build server
cargo build --release --package sqlrustgo-mysql-server

# Build test client
cargo build --release --package sqlrustgo-mysql-client --example hybrid_soak

# Verify binaries
ls -la target/release/sqlrustgo-mysql-server
ls -la target/release/examples/hybrid_soak
```

### 4. Prepare TPC-H Data

```bash
# Create data directory
mkdir -p /tmp/sqlrustgo-v311-soak

# Copy or generate TPC-H SF=0.1 data
# If you have existing data, scp from macOS:
# scp -r user@macbook:/tmp/sqlrustgo-v311-soak/* /tmp/sqlrustgo-v311-soak/

# Or load fresh using bulk_load_default.py
python3 /tmp/bulk_load_default.py --data-dir /tmp/sqlrustgo-v311-soak --tpch-sf 0.1
```

## Test Execution

### Start Server

```bash
cd /path/to/sqlrustgo

nohup ./target/release/sqlrustgo-mysql-server serve \
  --port 3396 \
  --data-dir /tmp/sqlrustgo-v311-soak \
  --storage binary \
  --server-threads 8 \
  --log-level error \
  > /tmp/soak_server.log 2>&1 &

echo "Server PID: $!"
sleep 3

# Verify server is running
lsof -i :3396 | grep LISTEN
```

### Start Client

```bash
cd /path/to/sqlrustgo

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

## Monitoring

### Check Status

```bash
# Check processes
ps aux | grep sqlrustgo
ps aux | grep hybrid_soak

# Check connections
lsof -i :3396

# Watch client output
tail -f /tmp/soak_client.log

# Get metrics
./docs/soak_status.sh
```

### Quick Status Report

```bash
#!/bin/bash
echo "=============================================="
echo "       SOAK Test Report - Ubuntu"
echo "=============================================="
echo "Time: $(date)"
echo ""

SERVER_PID=$(pgrep -f "sqlrustgo-mysql-server" | head -1)
CLIENT_PID=$(pgrep -f "hybrid_soak" | head -1)

LAST=$(grep "^\[" /tmp/soak_client.log 2>/dev/null | tail -1)
ELAPSED=$(echo "$LAST" | grep -oE "^\[ *[0-9]+" | grep -oE "[0-9]+" || echo 0)
HOURS=$(awk "BEGIN {printf \"%.1f\", $ELAPSED / 3600}")

QPS=$(echo "$LAST" | grep -oE "qps= *[0-9]+" | grep -oE "[0-9]+" || echo 0)
TOT_OPS=$(echo "$LAST" | grep -oE "tot_ops=[0-9]+" | cut -d= -f2 || echo 0)

SERVER_RSS=$(ps -p $SERVER_PID -o rss= 2>/dev/null | tr -d ' ' || echo 0)
SERVER_RSS_GB=$(awk "BEGIN {printf \"%.2f\", $SERVER_RSS / 1024 / 1024}")

CONNS=$(lsof -i :3396 2>/dev/null | grep ESTABLISHED | wc -l || echo 0)

echo "【Runtime】 ${HOURS}h (${ELAPSED}s)"
echo "【Server】 PID=$SERVER_PID RSS=${SERVER_RSS_GB}GB"
echo "【Client】 PID=$CLIENT_PID"
echo "【Connections】 $CONNS"
echo "【QPS】 $QPS/sec"
echo "【Total Ops】 $TOT_OPS"
echo ""
echo "【Last Report】"
echo "$LAST"
echo "=============================================="
```

## Troubleshooting

### Server Won't Start

```bash
# Check port
lsof -i :3396

# Check data directory
ls -la /tmp/sqlrustgo-v311-soak/

# View logs
cat /tmp/soak_server.log
```

### Connection Refused

```bash
# Verify server listening
lsof -i :3396 | grep LISTEN

# Test connection
mysql -h 127.0.0.1 -P 3396 -u root -e "SELECT 1"
```

### High Memory Usage

```bash
# Monitor memory
watch -n 5 'ps -p $SERVER_PID -o pid,rss,%mem'

# Check for memory leaks
valgrind --leak-check=full ./target/release/sqlrustgo-mysql-server
```

## Data Transfer (Optional)

To copy TPC-H data from macOS to Ubuntu:

```bash
# On macOS
scp -r /tmp/sqlrustgo-v311-soak user@ubuntu-host:/tmp/

# On Ubuntu
ls -la /tmp/sqlrustgo-v311-soak/
```

## Expected Results

| Metric | Expected |
|--------|----------|
| QPS | > 50/sec |
| Error Rate | < 1% |
| Memory | Stable (< 10GB RSS) |
| Runtime | 168 hours continuous |
