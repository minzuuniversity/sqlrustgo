#!/usr/bin/env bash
# run_binary_soak.sh — Binary storage SOAK runner (SF=0.1 → SF=1.0, 30m → 72h)
# Uses --storage binary for instant data load (no INSERT/LOAD DATA).
# Server + tpch_rotate + mixed SOAK run in background, survives SSH disconnect.
#
# Usage:
#   SF=0.1 PORT=3397 SERVER_THREADS=16 bash scripts/soak/run_binary_soak.sh
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# === Defaults ===
SF="${SF:-0.1}"              # 0.1 or 1.0
PORT="${PORT:-3397}"
SERVER_THREADS="${SERVER_THREADS:-0}"
TPCH_THREADS="${TPCH_THREADS:-32}"
DURATION_HOURS="${DURATION_HOURS:-72}"
MIX_RATIO="${MIX_RATIO:-80}"
CONC_MIN="${CONC_MIN:-16}"
CONC_MAX="${CONC_MAX:-64}"
SERVER_MEM_MB="${SERVER_MEM_MB:-0}"
TPCH_INTERVAL="${TPCH_INTERVAL:-120}"

# Binary data dirs (pre-built by tbl2bin)
BIN_DIR_SF01="$PROJECT_ROOT/data/tpch-sf01-bin"
BIN_DIR_SF1="$PROJECT_ROOT/data/tpch-sf1-bin"

# Resolve data dir
case "$SF" in
    0.1) DATA_DIR="$BIN_DIR_SF01" ;;
    1.0) DATA_DIR="$BIN_DIR_SF1" ;;
    *)   echo "FAIL: SF=$SF not supported (use 0.1 or 1.0)" >&2; exit 1 ;;
esac

RESULTS_DIR="${RESULTS_DIR:-$PROJECT_ROOT/soak_results/binary-sf${SF}-${DURATION_HOURS}h}"
mkdir -p "$RESULTS_DIR"

SQLRUSTGO_BIN="$PROJECT_ROOT/target/release/sqlrustgo-mysql-server"
PID_FILE="$RESULTS_DIR/server.pid"
LOG_FILE="$RESULTS_DIR/server.log"
METRICS_FILE="$RESULTS_DIR/metrics.csv"
TPCH_LOG="$RESULTS_DIR/tpch.log"
SOAK_LOG="$RESULTS_DIR/soak.log"
TPCH_STDOUT="$RESULTS_DIR/tpch-stdout.log"

echo "=========================================="
echo "Binary SOAK Runner"
echo "=========================================="
echo "  SF=$SF  DURATION=${DURATION_HOURS}h  PORT=$PORT"
echo "  SERVER_THREADS=$SERVER_THREADS  TPCH_THREADS=$TPCH_THREADS"
echo "  MIX_RATIO=$MIX_RATIO  CONC=[$CONC_MIN,$CONC_MAX]"
echo "  DATA_DIR=$DATA_DIR"
echo "  RESULTS=$RESULTS_DIR"
echo "=========================================="

# Check preconditions
if [ ! -d "$DATA_DIR" ]; then
    echo "FAIL: Binary data dir not found: $DATA_DIR"
    echo "Run: mkdir -p data/tpch-sf01-bin && ./target/release/tbl2bin data/tpch-sf01/ data/tpch-sf01-bin"
    exit 1
fi
if [ ! -x "$SQLRUSTGO_BIN" ]; then
    echo "FAIL: Binary not found: $SQLRUSTGO_BIN"
    exit 1
fi

# Check port free
if lsof -i ":$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "FAIL: port $PORT already in use"
    lsof -i ":$PORT"
    exit 1
fi

# === SERVER ===
echo "[1/5] Starting server (binary storage)..."
LIMIT_CMD=""
if [ "${SERVER_MEM_MB:-0}" -gt 0 ] 2>/dev/null; then
    LIMIT_CMD="ulimit -v $((SERVER_MEM_MB * 1024)); ulimit -n 4096;"
fi

{
    if [ -n "$LIMIT_CMD" ]; then
        eval "$LIMIT_CMD"
    fi
    exec "$SQLRUSTGO_BIN" serve \
        --port "$PORT" \
        --data-dir "$DATA_DIR" \
        --storage binary \
        --log-level info \
        --server-threads "$SERVER_THREADS"
} > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$PID_FILE"
echo "  Server PID=$SERVER_PID"

# Wait for server to be ready
for i in $(seq 1 30); do
    sleep 1
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "FAIL: server died on startup" >&2
        tail -20 "$LOG_FILE" >&2
        exit 1
    fi
    if lsof -i ":$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
        echo "  Server listening on $PORT after ${i}s"
        break
    fi
    [ "$i" -eq 30 ] && echo "FAIL: server not listening after 30s" && exit 1
done

# === METRICS CSV ===
echo "ts,elapsed_s,rss_mb,fd_count,cpu_pct,queries_done,crud_done,errors" > "$METRICS_FILE"

# === TPC-H ROTATE ===
echo "[2/5] Starting TPC-H 22 rotation..."
HOST=127.0.0.1 PORT="$PORT" INTERVAL="$TPCH_INTERVAL" \
    LOG_FILE="$TPCH_LOG" \
    bash "$PROJECT_ROOT/scripts/stability/tpch_22_rotate.sh" \
    >> "$TPCH_STDOUT" 2>&1 &
TPCH_PID=$!
echo "  TPC-H PID=$TPCH_PID"

# === MIXED SOAK DRIVER ===
echo "[3/5] Starting mixed SOAK driver (${DURATION_HOURS}h)..."
SOAK_DURATION_SECS=$((DURATION_HOURS * 3600))
python3 "$PROJECT_ROOT/scripts/soak/tpch_mixed_soak_driver.py" \
    --host=127.0.0.1 --port="$PORT" --user=root \
    --queries-dir="$PROJECT_ROOT/scripts/soak/tpch_queries" \
    --duration="$SOAK_DURATION_SECS" \
    --mix-ratio="$MIX_RATIO" \
    --concurrency-min="$CONC_MIN" --concurrency-max="$CONC_MAX" \
    --output-dir="$RESULTS_DIR" \
    >> "$SOAK_LOG" 2>&1 &
SOAK_PID=$!
echo "  SOAK PID=$SOAK_PID"

# === MONITORING LOOP ===
echo "[4/5] Monitoring loop (interval=60s, total=${DURATION_HOURS}h)..."
START_TS=$(date +%s)
END_TS=$((START_TS + SOAK_DURATION_SECS))
INITIAL_RSS=0
SAMPLE=0
SERVER_ALIVE=1

cleanup() {
    echo "[cleanup] Stopping..."
    kill -TERM "$SOAK_PID" "$TPCH_PID" 2>/dev/null || true
    sleep 2
    kill -TERM "$SERVER_PID" 2>/dev/null || true
    wait 2>/dev/null || true
    echo "[cleanup] Done at $(date)"
}
trap cleanup EXIT

while [ "$(date +%s)" -lt "$END_TS" ] && [ "$SERVER_ALIVE" -eq 1 ]; do
    sleep 60
    ELAPSED=$(($(date +%s) - START_TS))

    # Rotate server.log if > 500 MB
    LOG_BYTES=$(stat -c%s "$LOG_FILE" 2>/dev/null || echo 0)
    if [ "$LOG_BYTES" -gt 524288000 ]; then
        echo "$(date '+%Y-%m-%dT%H:%M:%S') rotating server.log (${LOG_BYTES}B)" >> "$LOG_FILE.rotated"
        : > "$LOG_FILE"
    fi

    # Check server alive
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "  CRASH: server died at elapsed=${ELAPSED}s"
        SERVER_ALIVE=0
        break
    fi

    # RSS, FD, CPU
    RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    RSS_MB=$((RSS_KB / 1024))
    FD_COUNT=$(lsof -p "$SERVER_PID" 2>/dev/null | wc -l | tr -d ' ' || echo 0)
    CPU_PCT=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' | cut -d. -f1 || echo 0)

    # Parse soak log for latest counts
    LAST_SOAK=$(grep -c '"queries_executed"' "$SOAK_LOG" 2>/dev/null || echo 0)
    LAST_ERRORS=$(grep 'Query errors:' "$SOAK_LOG" 2>/dev/null | tail -1 | grep -oE '[0-9]+' | tail -1 || echo 0)

    echo "$(date '+%Y-%m-%dT%H:%M:%S'),${ELAPSED},${RSS_MB},${FD_COUNT},${CPU_PCT},${LAST_SOAK},${LAST_ERRORS}" >> "$METRICS_FILE"

    REMAIN=$((END_TS - $(date +%s)))
    REMAIN_H=$((REMAIN / 3600))
    REMAIN_M=$(((REMAIN % 3600) / 60))
    echo "  [${ELAPSED}s / ${DURATION_HOURS}h] RSS=${RSS_MB}MB FD=${FD_COUNT} CPU=${CPU_PCT}% remain=${REMAIN_H}h${REMAIN_M}m"

    # RSS watchdog
    if [ "$SERVER_MEM_MB" -gt 0 ] 2>/dev/null; then
        if [ "$RSS_MB" -gt $((SERVER_MEM_MB * 4 / 5)) ] 2>/dev/null; then
            echo "  ALERT: RSS ${RSS_MB}MB > 80% of cap ${SERVER_MEM_MB}MB"
        fi
    fi
done

echo "[5/5] Waiting for SOAK/TPCH to finish..."
wait 2>/dev/null || true

trap - EXIT
cleanup

echo ""
echo "=========================================="
echo "Binary SOAK Complete — $SF @ ${DURATION_HOURS}h"
echo "Results: $RESULTS_DIR"
echo "Metrics: $METRICS_FILE ($(wc -l < "$METRICS_FILE") samples)"
echo "=========================================="
