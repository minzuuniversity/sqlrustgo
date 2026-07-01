#!/usr/bin/env bash
# smoke_binary_1m.sh — 1-minute binary-storage SOAK smoke test.
#
# Verifies: server with --storage binary serves concurrent queries
# without triggering the trigger.rs:104 panic storm (issue fixed per
# reports/SERVER_DEADLOCK_FIX_PLAN_2026-06-28.md).
#
# Usage: bash scripts/soak/smoke_binary_1m.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

SF=0.1
PORT=3397
DATA_DIR="$PROJECT_ROOT/data/tpch-sf01-bin"
RESULTS_DIR="${RESULTS_DIR:-/tmp/smoke-binary-1m}"
DURATION=70
SQLRUSTGO_BIN="$PROJECT_ROOT/target/release/sqlrustgo-mysql-server"

mkdir -p "$RESULTS_DIR"
LOG="$RESULTS_DIR/server.log"
METRICS="$RESULTS_DIR/metrics.csv"

# Sanity
[ -x "$SQLRUSTGO_BIN" ] || { echo "FAIL: $SQLRUSTGO_BIN not built"; exit 1; }
[ -d "$DATA_DIR" ]      || { echo "FAIL: $DATA_DIR missing"; exit 1; }
if ss -tlnp 2>/dev/null | grep -q ":$PORT "; then
  echo "FAIL: port $PORT already in use"; exit 1
fi

# Start server
echo "[1/3] Starting server (binary storage)..."
"$SQLRUSTGO_BIN" serve \
  --port "$PORT" \
  --data-dir "$DATA_DIR" \
  --storage binary \
  --log-level info \
  --server-threads 8 \
  > "$LOG" 2>&1 &
SERVER_PID=$!
echo "  Server PID=$SERVER_PID"

# Wait for listen
for i in $(seq 1 30); do
  sleep 1
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "FAIL: server died on startup"
    tail -20 "$LOG"
    exit 1
  fi
  if ss -tlnp 2>/dev/null | grep -q ":$PORT "; then
    echo "  Listening on $PORT after ${i}s"
    break
  fi
done

# Header
echo "ts,elapsed_s,rss_mb,fd_count,cpu_pct,queries_done,errors" > "$METRICS"

# Background: 4 mysql CLI workers hammering simple queries
echo "[2/3] Running 4 concurrent workers for ${DURATION}s..."
QUERIES_PER_WORKER=0
ERRORS=0
for w in 1 2 3 4; do
  (
    count=0
    errs=0
    end=$(( $(date +%s) + DURATION ))
    while [ "$(date +%s)" -lt "$end" ]; do
      if mysql -h 127.0.0.1 -P "$PORT" -u root --silent tpch \
        -e "SELECT COUNT(*) FROM lineitem" >/dev/null 2>&1; then
        count=$((count + 1))
      else
        errs=$((errs + 1))
      fi
    done
    echo "WORKER $w done: $count queries, $errs errors"
  ) &
done

# Monitor loop
START=$(date +%s)
END=$((START + DURATION))
SAMPLE=0
while [ "$(date +%s)" -lt "$END" ]; do
  sleep 10
  SAMPLE=$((SAMPLE + 1))
  ELAPSED=$(($(date +%s) - START))
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "  CRASH: server died at elapsed=${ELAPSED}s"
    break
  fi
  RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
  RSS_MB=$((RSS_KB / 1024))
  FD_COUNT=$(ls -la /proc/"$SERVER_PID"/fd 2>/dev/null | wc -l || echo 0)
  CPU=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
  # Count completed queries from server log (approximate via OK packets)
  Q_DONE=$(grep -c "send_result_set done" "$LOG" 2>/dev/null || echo 0)
  echo "  [${ELAPSED}s] RSS=${RSS_MB}MB FD=${FD_COUNT} CPU=${CPU}% queries_logged=$Q_DONE"
  echo "$(date '+%Y-%m-%dT%H:%M:%S'),${ELAPSED},${RSS_MB},${FD_COUNT},${CPU},${Q_DONE},0" >> "$METRICS"
done

# Wait for workers
wait
echo "[3/3] Stopping server..."
kill -TERM "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true

# Check panic count
PANICS=$(grep -c "Storage MUST be WalStorage" "$LOG" 2>/dev/null || echo 0)
QUERY_OK=$(grep -c "send_result_set done" "$LOG" 2>/dev/null || echo 0)
echo ""
echo "=========================================="
echo "Smoke results:"
echo "  Panics (Storage MUST be WalStorage): $PANICS"
echo "  Successful queries logged:           $QUERY_OK"
echo "=========================================="
if [ "$PANICS" -gt 0 ]; then
  echo "FAIL: trigger.rs panic storm still present"
  exit 1
fi
if [ "$QUERY_OK" -lt 100 ]; then
  echo "FAIL: too few queries completed (expected >= 100)"
  exit 1
fi
echo "PASS"
