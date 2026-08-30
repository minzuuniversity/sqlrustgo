#!/bin/bash
# run_168h_ci.sh — 168h SOAK runner for Gitea CI / Z6G4 Docker container
#
# V312-59-D GA-2: 168h mixed SOAK against sqlrustgo-mysql-server.
# Uses MySQL CLI + the canonical sysbench oltp_read_write harness from
# scripts/soak/run_soak_loop.sh (which already passes --db-ps-mode=disable
# to sidestep the 4/8 TLS-handshake client-side stall under investigation
# in #4564).
#
# Designed to run inside the Z6G4 Alpine container built from
# scripts/soak/Dockerfile.soak.
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
#   MYSQL_BIN     path to mysql CLI (default: mysql)
#   BINARY        path to sqlrustgo-mysql-server (default: ./target/release/sqlrustgo-mysql-server)

set -euo pipefail

PORT=${PORT:-3397}
DATA_DIR=${DATA_DIR:-/tmp/sqlrustgo-168h}
RESULTS_DIR=${RESULTS_DIR:-$(pwd)/results/soak-168h}
SERVER_THREADS=${SERVER_THREADS:-16}
HOURS=${HOURS:-168}
MYSQL_BIN=${MYSQL_BIN:-mysql}
BINARY=${BINARY:-./target/release/sqlrustgo-mysql-server}

END_TS=$(( $(date +%s) + HOURS * 3600 ))
START_TS=$(date +%s)

mkdir -p "$DATA_DIR" "$RESULTS_DIR"

echo "=== 168h SOAK Start ==="
echo "Binary: $BINARY"
echo "Port: $PORT  Data: $DATA_DIR  Results: $RESULTS_DIR"
echo "Duration: ${HOURS}h (until $(date -d @$END_TS))"
echo "Server threads: $SERVER_THREADS"
echo "MySQL CLI: $MYSQL_BIN"
echo ""

# ── 1. Start server ──
"$BINARY" serve \
  --port "$PORT" \
  --data-dir "$DATA_DIR" \
  --auth-mode none \
  --server-threads "$SERVER_THREADS" \
  --max-connections 200 \
  --metrics-port 9300 \
  --log-level info \
  > "$RESULTS_DIR/server.log" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$RESULTS_DIR/server.pid"
echo "Server PID: $SERVER_PID"

# ── 2. Wait for server ready (using mysql ping, not binary subcommand) ──
echo -n "Waiting for server..."
READY=0
for i in $(seq 1 30); do
  if "$MYSQL_BIN" -h 127.0.0.1 -P "$PORT" -uroot --silent -e "SELECT 1" > /dev/null 2>&1; then
    echo " ready after ${i}s"
    READY=1
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo " FAILED"
    tail -50 "$RESULTS_DIR/server.log"
    exit 1
  fi
  sleep 2
  echo -n "."
done
[ "$READY" = "1" ] || exit 1

# ── 3. Initialize schema (via mysql CLI) ──
echo "Creating schema..."
# Only create the database here. Table creation is delegated entirely to
# `sysbench prepare` below (its oltp_read_write schema owns sbtest1..15).
# A pre-created sbtest1 here conflicts with sysbench's own CREATE TABLE
# (error 1105 "already exists") — see PR fix/v312-60-168h-schema-conflict.
"$MYSQL_BIN" -h 127.0.0.1 -P "$PORT" -uroot --silent \
  -e "CREATE DATABASE IF NOT EXISTS sbtest" || true

echo "Schema ready (tables owned by sysbench prepare)"

# ── 4. sysbench prepare (use --db-ps-mode=disable per run_soak_loop.sh workaround) ──
echo "=== sysbench prepare (--db-ps-mode=disable) ==="
sysbench oltp_read_write \
  --db-driver=mysql \
  --mysql-host=127.0.0.1 --mysql-port="$PORT" \
  --mysql-user=root --mysql-db=sbtest \
  --table-size=10000 --tables=1 \
  prepare > "$RESULTS_DIR/sysbench_prepare.log" 2>&1 || {
  echo "sysbench prepare failed"
  tail -30 "$RESULTS_DIR/sysbench_prepare.log"
  exit 1
}

# ── 5. Launch sysbench run in background (with --db-ps-mode=disable) ──
echo "=== sysbench run (background, ${HOURS}h) ==="
sysbench oltp_read_write \
  --db-driver=mysql \
  --db-ps-mode=disable \
  --mysql-host=127.0.0.1 --mysql-port="$PORT" \
  --mysql-user=root --mysql-db=sbtest \
  --table-size=10000 --tables=1 \
  --threads=8 --time=$((HOURS * 3600)) --report-interval=600 \
  run > "$RESULTS_DIR/sysbench_run.log" 2>&1 &
SYSBENCH_PID=$!
echo "$SYSBENCH_PID" > "$RESULTS_DIR/sysbench.pid"
echo "Sysbench PID: $SYSBENCH_PID"

# ── 6. Metrics header ──
METRICS="$RESULTS_DIR/metrics.csv"
echo "ts,elapsed_s,rss_kb,fd,threads,wal_bytes,disk_avail_kb,queries_ok,server_alive,sysbench_alive" > "$METRICS"
QUERIES_OK=0

# ── 7. SOAK monitor loop (1 sample/min) ──
echo "=== SOAK monitor loop started (${HOURS}h) ==="
SAMPLE=0
while [ $(date +%s) -lt $END_TS ]; do
  SAMPLE=$((SAMPLE + 1))
  TS=$(date +%s)
  ELAPSED=$((TS - START_TS))

  # Server alive check
  ALIVE=1
  if ! kill -0 $SERVER_PID 2>/dev/null; then
    ALIVE=0
    echo "!!! SERVER DIED at elapsed=${ELAPSED}s" | tee -a "$RESULTS_DIR/errors.log"
    tail -100 "$RESULTS_DIR/server.log" >> "$RESULTS_DIR/errors.log"
    exit 1
  fi

  # Sysbench alive check
  SB_ALIVE=1
  if ! kill -0 $SYSBENCH_PID 2>/dev/null; then
    SB_ALIVE=0
    echo "!!! SYSBENCH DIED at elapsed=${ELAPSED}s" | tee -a "$RESULTS_DIR/errors.log"
    tail -30 "$RESULTS_DIR/sysbench_run.log" >> "$RESULTS_DIR/errors.log"
    # Don't exit — server can keep running; sysbench crash is data
  fi

  # Resource metrics
  RSS=$(ps -o rss= -p $SERVER_PID 2>/dev/null | tr -d ' ' || echo 0)
  FD=$(ls /proc/$SERVER_PID/fd 2>/dev/null | wc -l || echo 0)
  THREADS=$(ps -o nlwp= -p $SERVER_PID 2>/dev/null | tr -d ' ' || echo 0)
  WAL=$(du -sb "$DATA_DIR" 2>/dev/null | cut -f1 || echo 0)
  DISK=$(df "$DATA_DIR" 2>/dev/null | tail -1 | awk '{print $4}' || echo 0)

  # Query probe (use mysql CLI, not binary exec)
  if "$MYSQL_BIN" -h 127.0.0.1 -P "$PORT" -uroot --silent \
       -e "SELECT ${SAMPLE}" > /dev/null 2>&1; then
    QUERIES_OK=$((QUERIES_OK + 1))
  fi

  echo "$TS,$ELAPSED,$RSS,$FD,$THREADS,$WAL,$DISK,$QUERIES_OK,$ALIVE,$SB_ALIVE" >> "$METRICS"

  # Status print every 60 samples (60 min)
  if [ $((SAMPLE % 60)) -eq 0 ]; then
    TPS=$((QUERIES_OK / (ELAPSED / 60 + 1)))
    echo "[$(date -u)] h=$((ELAPSED/3600)).$(((ELAPSED%3600)/60)) \
rss=${RSS}KB fd=$FD thr=$THREADS wal=${WAL} q=${QUERIES_OK} tps=${TPS}/m \
server_alive=$ALIVE sysbench_alive=$SB_ALIVE"
  fi

  # Snapshot every 6h (360 samples)
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
- **Server alive**: $ALIVE
- **Sysbench alive**: $SB_ALIVE
EOF
    tar czf "$RESULTS_DIR/snapshot_${ELAPSED}s.tar.gz" \
      -C "$RESULTS_DIR" metrics.csv server.log sysbench_run.log errors.log 2>/dev/null || true
  fi

  sleep 60
done

# ── 8. Stop sysbench, let it finish gracefully ──
echo "=== SOAK wall time elapsed; stopping sysbench ==="
if kill -0 $SYSBENCH_PID 2>/dev/null; then
  kill -INT $SYSBENCH_PID 2>/dev/null || true
  wait $SYSBENCH_PID 2>/dev/null || true
fi
echo "Sysbench final report:"
tail -30 "$RESULTS_DIR/sysbench_run.log"

# ── 9. Final report ──
TOTAL_SEC=$(( $(date +%s) - START_TS ))
cat > "$RESULTS_DIR/SOAK_168H_REPORT.md" <<EOF
# 168h SOAK Report

- **Started**: $(date -d @$START_TS -u)
- **Completed**: $(date -u)
- **Wall time**: ${TOTAL_SEC}s ($((TOTAL_SEC/3600))h $(((TOTAL_SEC%3600)/60))m)
- **Samples**: $SAMPLE
- **Queries OK**: $QUERIES_OK
- **Avg probe TPS**: $((QUERIES_OK / (TOTAL_SEC/60 + 1))) /min

## Resource Metrics (peak | final)

| Metric | Peak | Final |
|--------|------|-------|
| RSS | $(awk -F, 'NR>1 && $3>m{m=$3} END{print m}' "$METRICS") KB | $(tail -1 "$METRICS" | cut -d, -f3) KB |
| FD | $(awk -F, 'NR>1 && $4>m{m=$4} END{print m}' "$METRICS") | $(tail -1 "$METRICS" | cut -d, -f4) |
| Threads | $(awk -F, 'NR>1 && $5>m{m=$5} END{print m}' "$METRICS") | $(tail -1 "$METRICS" | cut -d, -f5) |
| WAL | $(awk -F, 'NR>1 && $6>m{m=$6} END{print m}' "$METRICS") bytes | $(tail -1 "$METRICS" | cut -d, -f6) bytes |

## Sysbench summary

See \`sysbench_run.log\` (last 30 lines above).
EOF

cat "$RESULTS_DIR/SOAK_168H_REPORT.md"