#!/usr/bin/env bash
# pymysql_30min_soak.sh — 30-min wired stability soak (pymysql client).
#
# Bypasses the mysql 8.0 CLI TLS SELECT-hang (see ISSUE-tls-select-hang.md)
# by using pymysql. Mirrors scripts/soak/smoke_binary_1m.sh structure but
# runs for 30 minutes against the binary-storage SF=0.1 dataset.
#
# Usage:
#   bash scripts/soak/pymysql_30min_soak.sh [DURATION_MIN=30] [PORT=3396]
#
# Exit codes:
#   0  PASS — server alive, no panics, queries done > threshold
#   1  precondition failure (bin missing, port in use, etc.)
#   2  server crashed during run

set -uo pipefail

DURATION_MIN="${1:-30}"
DURATION=$((DURATION_MIN * 60))
PORT="${2:-3396}"
DATA_DIR="$PWD/data/tpch-sf01-bin"
RESULTS_DIR="${RESULTS_DIR:-$PWD/soak_results/pymysql-${DURATION_MIN}min-$(date +%Y%m%d_%H%M%S)}"
SQLRUSTGO_BIN="$PWD/target/release/sqlrustgo-mysql-server"
THREADS="${THREADS:-16}"
SERVER_THREADS="${SERVER_THREADS:-16}"
SAMPLE_INTERVAL="${SAMPLE_INTERVAL:-15}"

mkdir -p "$RESULTS_DIR"
LOG="$RESULTS_DIR/server.log"
METRICS="$RESULTS_DIR/metrics.csv"
DRIVER_LOG="$RESULTS_DIR/driver.log"

cd "$PWD"

# Sanity
[ -x "$SQLRUSTGO_BIN" ] || { echo "FAIL: $SQLRUSTGO_BIN missing" >&2; exit 1; }
[ -d "$DATA_DIR" ]      || { echo "FAIL: $DATA_DIR missing" >&2; exit 1; }
if ss -tlnp 2>/dev/null | grep -q ":$PORT "; then
  echo "FAIL: port $PORT in use" >&2; exit 1
fi
python3 -c 'import pymysql' 2>/dev/null || {
  pip3 install --user pymysql 2>&1 | tail -3
}

echo "================================================"
echo "pymysql ${DURATION_MIN}min Wired SOAK"
echo "================================================"
echo "  PORT=$PORT  THREADS=$THREADS  SERVER_THREADS=$SERVER_THREADS"
echo "  DATA_DIR=$DATA_DIR  RESULTS_DIR=$RESULTS_DIR"
echo "================================================"

# --- start server ---
echo "[1/4] Starting sqlrustgo-mysql-server (binary storage)..."
"$SQLRUSTGO_BIN" serve \
  --port "$PORT" \
  --data-dir "$DATA_DIR" \
  --storage binary \
  --log-level info \
  --server-threads "$SERVER_THREADS" \
  > "$LOG" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$RESULTS_DIR/server.pid"
echo "  server PID=$SERVER_PID"

for i in $(seq 1 30); do
  sleep 1
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "FAIL: server died on startup" >&2; tail -20 "$LOG"; exit 1
  fi
  if ss -tlnp 2>/dev/null | grep -q ":$PORT "; then
    echo "  Listening on $PORT after ${i}s"; break
  fi
done

echo "ts,elapsed_s,rss_mb,fd_count,cpu_pct,queries_done,errors" > "$METRICS"

# --- launch pymysql driver ---
echo "[2/4] Starting pymysql driver (${THREADS} threads, ${DURATION}s)..."
python3 "$PWD/scripts/soak/pymysql_driver.py" \
  --host=127.0.0.1 --port="$PORT" --threads="$THREADS" --duration="$DURATION" \
  --output="$DRIVER_LOG" &
DRIVER_PID=$!
echo "  driver PID=$DRIVER_PID"

# --- monitor loop ---
echo "[3/4] Monitoring (interval=${SAMPLE_INTERVAL}s)..."
START_TS=$(date +%s)
END_TS=$((START_TS + DURATION))
SERVER_ALIVE=1
SAMPLE=0
while [ "$(date +%s)" -lt "$END_TS" ] && [ "$SERVER_ALIVE" -eq 1 ]; do
  sleep "$SAMPLE_INTERVAL"
  SAMPLE=$((SAMPLE + 1))
  ELAPSED=$(($(date +%s) - START_TS))
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "  CRASH: server died at elapsed=${ELAPSED}s"
    SERVER_ALIVE=0
    break
  fi
  RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
  RSS_MB=$((RSS_KB / 1024))
  FD_COUNT=$(ls -la /proc/"$SERVER_PID"/fd 2>/dev/null | wc -l | tr -d ' ' || echo 0)
  CPU=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
  Q_DONE=$(grep -c "send_result_set done" "$LOG" 2>/dev/null || echo 0)
  PANIC_COUNT=$(grep -c "Storage MUST be WalStorage" "$LOG" 2>/dev/null || echo 0)
  REMAIN=$(( (END_TS - $(date +%s)) / 60 ))
  echo "  [${ELAPSED}s/${DURATION}s] RSS=${RSS_MB}MB FD=${FD_COUNT} CPU=${CPU}% q_logged=${Q_DONE} panics=${PANIC_COUNT} remain=${REMAIN}m"
  echo "$(date '+%Y-%m-%dT%H:%M:%S'),${ELAPSED},${RSS_MB},${FD_COUNT},${CPU},${Q_DONE},${PANIC_COUNT}" >> "$METRICS"
done

# --- cleanup ---
echo "[4/4] Stopping driver + server..."
kill -TERM "$DRIVER_PID" 2>/dev/null || true
wait "$DRIVER_PID" 2>/dev/null || true
kill -TERM "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true

# --- summary ---
PANIC_COUNT=$(grep -c "Storage MUST be WalStorage" "$LOG" 2>/dev/null || echo 0)
Q_DONE=$(grep -c "send_result_set done" "$LOG" 2>/dev/null || echo 0)
DRIVER_Q=$(grep -c '^Q,' "$DRIVER_LOG" 2>/dev/null || echo 0)
DRIVER_E=$(grep -c '^E,' "$DRIVER_LOG" 2>/dev/null || echo 0)
{
echo "# pymysql ${DURATION_MIN}min Wired SOAK Report"
echo
echo "**Date**: $(date -u +'%Y-%m-%dT%H:%M:%SZ')"
echo "**Duration**: ${DURATION_MIN}min (${DURATION}s)"
echo "**Port**: $PORT"
echo "**Threads**: $THREADS  (server worker threads: $SERVER_THREADS)"
echo "**Data dir**: $DATA_DIR"
echo
echo "## Metrics"
echo
echo "| Metric | Value |"
echo "|--------|-------|"
echo "| Server PID | $SERVER_PID |"
echo "| Driver PID | $DRIVER_PID |"
echo "| Server alive at end | $SERVER_ALIVE |"
echo "| send_result_set done (server log) | $Q_DONE |"
echo "| Driver queries done | $DRIVER_Q |"
echo "| Driver errors | $DRIVER_E |"
echo "| Trigger panics (Storage MUST be WalStorage) | $PANIC_COUNT |"
echo
echo "## Verdict"
echo
} > "$RESULTS_DIR/STABILITY_REPORT.md"

if [ "$SERVER_ALIVE" -eq 0 ]; then
  echo "FAIL — server crashed during the soak run." | tee -a "$RESULTS_DIR/STABILITY_REPORT.md"
  exit 2
fi
if [ "$PANIC_COUNT" -gt 0 ]; then
  echo "FAIL — panic storm detected ($PANIC_COUNT occurrences)." | tee -a "$RESULTS_DIR/STABILITY_REPORT.md"
  exit 2
fi
if [ "$DRIVER_Q" -lt 100 ]; then
  echo "FAIL — driver completed fewer than 100 queries ($DRIVER_Q)." | tee -a "$RESULTS_DIR/STABILITY_REPORT.md"
  exit 2
fi
echo "PASS — ${DRIVER_Q} queries via ${THREADS} threads in ${DURATION_MIN}min, no server crash, no panics." \
  | tee -a "$RESULTS_DIR/STABILITY_REPORT.md"
exit 0
