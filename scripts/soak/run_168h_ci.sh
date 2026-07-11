#!/bin/bash
# run_168h_ci.sh — 168h SOAK runner for Gitea CI (no external deps)
# Uses sqlrustgo's built-in `soak` subcommand + `serve` subcommand.
# Designed to run inside Z6G4 Docker container (Alpine).
#
# Usage:
#   PORT=3397 DATA_DIR=/tmp/sqlrustgo-data ./run_168h_ci.sh
#
# Env:
#   PORT          server port (default: 3397)
#   DATA_DIR      data directory (default: /tmp/sqlrustgo-168h)
#   RESULTS_DIR   output dir (default: ./results/soak-168h)
#   SERVER_THREADS (default: 16)
#   HOURS         duration (default: 168)

set -euo pipefail

PORT=${PORT:-3397}
DATA_DIR=${DATA_DIR:-/tmp/sqlrustgo-168h}
RESULTS_DIR=${RESULTS_DIR:-$(pwd)/results/soak-168h}
SERVER_THREADS=${SERVER_THREADS:-16}
HOURS=${HOURS:-168}

BINARY="./target/release/sqlrustgo"
END_TS=$(( $(date +%s) + HOURS * 3600 ))
START_TS=$(date +%s)

mkdir -p "$DATA_DIR" "$RESULTS_DIR"

echo "=== 168h SOAK Start ==="
echo "Binary: $BINARY"
echo "Port: $PORT  Data: $DATA_DIR  Results: $RESULTS_DIR"
echo "Duration: ${HOURS}h (until $(date -d @$END_TS))"
echo "Server threads: $SERVER_THREADS"

# ── 1. Start server ──
"$BINARY" serve \
  --port "$PORT" \
  --data-dir "$DATA_DIR" \
  --tls off \
  --server-threads "$SERVER_THREADS" \
  --max-connections 200 \
  --log-level info \
  > "$RESULTS_DIR/server.log" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$RESULTS_DIR/server.pid"
echo "Server PID: $SERVER_PID"

# Wait for server ready
echo -n "Waiting for server..."
for i in $(seq 1 30); do
  if echo "SELECT 1" | "$BINARY" soak --port "$PORT" --user root > /dev/null 2>&1; then
    echo " ready after ${i}s"
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo " FAILED"
    tail -30 "$RESULTS_DIR/server.log"
    exit 1
  fi
  sleep 2
  echo -n "."
done

# ── 2. Initialize schema ──
echo "Creating schema..."
echo "CREATE DATABASE IF NOT EXISTS sbtest" | "$BINARY" soak --port "$PORT" --user root || true
echo "CREATE TABLE IF NOT EXISTS sbtest.sbtest1 (
  id INT PRIMARY KEY AUTO_INCREMENT,
  k INT DEFAULT 0,
  c VARCHAR(100) DEFAULT '',
  pad VARCHAR(100) DEFAULT ''
)" | "$BINARY" soak --port "$PORT" --user root || true
echo "Schema ready"

# ── 3. Metrics header ──
METRICS="$RESULTS_DIR/metrics.csv"
echo "ts,elapsed_s,rss_kb,fd,threads,wal_bytes,disk_avail_kb,queries_ok,server_alive" > "$METRICS"
QUERIES_OK=0

# ── 4. SOAK monitor loop ──
echo "=== SOAK loop started (${HOURS}h) ==="
SAMPLE=0
while [ $(date +%s) -lt $END_TS ]; do
  SAMPLE=$((SAMPLE + 1))
  TS=$(date +%s)
  ELAPSED=$((TS - START_TS))

  # Check server alive
  ALIVE=1
  if ! kill -0 $SERVER_PID 2>/dev/null; then
    ALIVE=0
    echo "!!! SERVER DIED at elapsed=${ELAPSED}s" | tee -a "$RESULTS_DIR/errors.log"
    wait $SERVER_PID 2>/dev/null || true
    echo "Exit code: $?" >> "$RESULTS_DIR/errors.log"
    tail -50 "$RESULTS_DIR/server.log" >> "$RESULTS_DIR/errors.log"
    exit 1
  fi

  # Resource metrics
  RSS=$(ps -o rss= -p $SERVER_PID 2>/dev/null | tr -d ' ' || echo 0)
  FD=$(ls /proc/$SERVER_PID/fd 2>/dev/null | wc -l || echo 0)
  THREADS=$(ps -o nlwp= -p $SERVER_PID 2>/dev/null | tr -d ' ' || echo 0)
  WAL=$(du -sb "$DATA_DIR" 2>/dev/null | cut -f1 || echo 0)
  DISK=$(df / 2>/dev/null | tail -1 | awk '{print $4}' || echo 0)

  # Query check
  if echo "SELECT $SAMPLE" | "$BINARY" soak --port "$PORT" --user root > /dev/null 2>&1; then
    QUERIES_OK=$((QUERIES_OK + 1))
  fi

  echo "$TS,$ELAPSED,$RSS,$FD,$THREADS,$WAL,$DISK,$QUERIES_OK,$ALIVE" >> "$METRICS"

  # Status every 30 samples (30 min)
  if [ $((SAMPLE % 30)) -eq 0 ]; then
    TPS=$((QUERIES_OK / (ELAPSED / 60 + 1)))
    echo "[$(date -u)] h=$((ELAPSED/3600)).$(((ELAPSED%3600)/60)) \
rss=${RSS}KB fd=$FD thr=$THREADS wal=$WAL q=${QUERIES_OK} tps=${TPS}/m"
  fi

  # Archive snapshot every 6h
  if [ $((SAMPLE % 360)) -eq 0 ]; then
    cp "$METRICS" "$RESULTS_DIR/metrics_${ELAPSED}s.csv"
    cat > "$RESULTS_DIR/STATUS_${ELAPSED}s.md" <<EOF
## Status at ${ELAPSED}s ($((ELAPSED/3600))h)

- **Duration**: $((ELAPSED/3600))h $(((ELAPSED%3600)/60))m
- **RSS**: ${RSS}KB ($((RSS/1024))MB)
- **FD**: $FD
- **Threads**: $THREADS
- **WAL**: $WAL bytes
- **Disk available**: $((DISK/1024))MB
- **Queries OK**: $QUERIES_OK
- **TPS (avg)**: $TPS /min
- **Server alive**: $ALIVE
EOF
    tar czf "$RESULTS_DIR/snapshot_${ELAPSED}s.tar.gz" \
      -C "$RESULTS_DIR" metrics.csv server.log errors.log 2>/dev/null || true
  fi

  sleep 60
done

# ── 5. Final report ──
echo ""
echo "=== 168h SOAK COMPLETE ==="
TOTAL_SEC=$(( $(date +%s) - START_TS ))
cat > "$RESULTS_DIR/SOAK_168H_REPORT.md" <<EOF
# 168h SOAK Report

- **Started**: $(date -d @$START_TS -u)
- **Completed**: $(date -u)
- **Wall time**: ${TOTAL_SEC}s ($((TOTAL_SEC/3600))h $(((TOTAL_SEC%3600)/60))m)
- **Samples**: $SAMPLE
- **Queries OK**: $QUERIES_OK
- **Avg TPS**: $((QUERIES_OK / (TOTAL_SEC/60 + 1))) /min
- **Server exits**: 0

## Resource Metrics

| Metric | Peak | Final |
|--------|------|-------|
| RSS | $(awk -F, 'NR>1 && \$3>m{m=\$3} END{print m}' "$METRICS") KB | $(tail -1 "$METRICS" | cut -d, -f3) KB |
| FD | $(awk -F, 'NR>1 && \$4>m{m=\$4} END{print m}' "$METRICS") | $(tail -1 "$METRICS" | cut -d, -f4) |
| Threads | $(awk -F, 'NR>1 && \$5>m{m=\$5} END{print m}' "$METRICS") | $(tail -1 "$METRICS" | cut -d, -f5) |
| WAL | $(awk -F, 'NR>1 && \$6>m{m=\$6} END{print m}' "$METRICS") bytes | $(tail -1 "$METRICS" | cut -d, -f6) bytes |
EOF
cat "$RESULTS_DIR/SOAK_168H_REPORT.md"
