#!/bin/bash
# run_tpch_30min.sh - 30-min TPC-H 22 wired soak (sysbench-free)
#
# Used when sysbench 1.0.20 lacks oltp_read_write.lua (minimal install).
# Runs the wired stack with TPC-H 22 query rotation as the SOLE workload.
#
# Not a replacement for run_wired_soak.sh — that script remains the
# canonical driver. This is a fallback for the sysbench-missing case.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

DURATION=${DURATION:-1800}            # 30 min default
INTERVAL=${INTERVAL:-5}              # 5s sample interval
TPCH_INTERVAL=${TPCH_INTERVAL:-60}   # 60s TPC-H round
TPCH_MAX_ROUNDS=${TPCH_MAX_ROUNDS:-30}
PORT=${PORT:-3400}
HOST=${HOST:-127.0.0.1}
SQLRUSTGO_BIN=${SQLRUSTGO_BIN:-./target/release/sqlrustgo-mysql-server}
FIXTURE=${FIXTURE:-tpch-sf001}

RESULTS_DIR="test_results/tpch_30min_$(date +%Y%m%d_%H%M%S)"
PID_FILE="$RESULTS_DIR/sqlrustgo.pid"
LOG_FILE="$RESULTS_DIR/sqlrustgo.log"
METRICS_FILE="$RESULTS_DIR/metrics.csv"
TPCH_LOG="$RESULTS_DIR/tpch_22_rotate.log"
TPCH_PID_FILE="$RESULTS_DIR/tpch_rotate.pid"

mkdir -p "$RESULTS_DIR"
DATA_DIR="$RESULTS_DIR/data"
mkdir -p "$DATA_DIR"

echo "=========================================="
echo "SQLRustGo 30-min TPC-H wired soak (sysbench-free)"
echo "=========================================="
echo "Duration: ${DURATION}s  Interval: ${INTERVAL}s  TPCH_INTERVAL: ${TPCH_INTERVAL}s"
echo "Port: $PORT  Host: $HOST  Data: $DATA_DIR"
echo "FIXTURE: $FIXTURE  Binary: $SQLRUSTGO_BIN"
echo "Results: $RESULTS_DIR"
echo "=========================================="

# Pre-flight
if [ ! -x "$SQLRUSTGO_BIN" ]; then
    echo "FAIL: $SQLRUSTGO_BIN not executable" >&2; exit 1
fi
if ! command -v mysql >/dev/null 2>&1; then
    echo "FAIL: mysql CLI not in PATH" >&2; exit 1
fi
if lsof -i ":$PORT" >/dev/null 2>&1; then
    echo "FAIL: port $PORT already in use" >&2; lsof -i ":$PORT"; exit 1
fi

# [1] Launch server
echo ""
echo "[1/4] Starting sqlrustgo-mysql-server..."
nohup "$SQLRUSTGO_BIN" serve \
    --host "$HOST" --port "$PORT" \
    --data-dir "$DATA_DIR" \
    --log-level info \
    > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$PID_FILE"
echo "  Server PID=$SERVER_PID"

# Wait for ready
for i in 1 2 3 4 5 6 7 8 9 10; do
    sleep 1
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "FAIL: server died on startup" >&2; cat "$LOG_FILE" >&2; exit 1
    fi
    if lsof -i ":$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
        echo "  Server listening on $PORT after ${i}s"
        break
    fi
done

# [2] Load TPC-H fixture (prefer INSERT mode — works on all v3.9.0 binaries;
#    LOAD DATA mode requires recent PR-3233+ binary with fixed data_dir path)
echo ""
echo "[2/4] Loading TPC-H fixture: $FIXTURE (INSERT mode) ..."
LOADER="${TPC_H_LOADER:-$SCRIPT_DIR/load_tpch_fixture_insert.sh}"
if [ -f "$LOADER" ]; then
    HOST="$HOST" PORT="$PORT" FIXTURE="$FIXTURE" \
        bash "$LOADER" || {
        echo "WARN: fixture load failed; continuing (queries may error)" >&2
    }
else
    echo "  WARN: $LOADER missing; skipping" >&2
fi

# [3] Launch TPC-H rotation
echo ""
echo "[3/4] Launching TPC-H 22-query rotation (interval=${TPCH_INTERVAL}s, max_rounds=$TPCH_MAX_ROUNDS)..."
HOST="$HOST" PORT="$PORT" \
    INTERVAL="$TPCH_INTERVAL" \
    MAX_ROUNDS="$TPCH_MAX_ROUNDS" \
    LOG_FILE="$TPCH_LOG" \
    nohup bash "$SCRIPT_DIR/tpch_22_rotate.sh" > "$RESULTS_DIR/tpch_rotate.stdout" 2>&1 &
TPCH_ROTATE_PID=$!
echo "$TPCH_ROTATE_PID" > "$TPCH_PID_FILE"
echo "  TPC-H rotate PID=$TPCH_ROTATE_PID  log=$TPCH_LOG"

# [4] Monitoring loop
echo ""
echo "[4/4] Monitoring loop (interval=${INTERVAL}s, total=${DURATION}s)..."
echo "ts,elapsed_s,rss_mb,rss_delta_mb,fd_count,fd_delta,cpu_pct,wal_mb,wal_files,lock_count,server_alive,tpch_round" > "$METRICS_FILE"

START_TS=$(date +%s)
END_TS=$((START_TS + DURATION))
INITIAL_RSS=0
INITIAL_FD=0
INITIAL_WAL=0
SAMPLE_COUNT=0
CRASH_DETECTED=0

cleanup() {
    echo ""
    echo "[cleanup] Stopping TPC-H rotate (PID $TPCH_ROTATE_PID)..."
    kill "$TPCH_ROTATE_PID" 2>/dev/null || true
    wait "$TPCH_ROTATE_PID" 2>/dev/null || true
    echo "[cleanup] Stopping server (PID $SERVER_PID)..."
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
    echo "[cleanup] Done"
}
trap cleanup EXIT

while [ "$(date +%s)" -lt "$END_TS" ]; do
    TS=$(date '+%Y-%m-%d %H:%M:%S')
    ELAPSED=$(($(date +%s) - START_TS))
    SAMPLE_COUNT=$((SAMPLE_COUNT + 1))

    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "  CRASH: server died at $TS (elapsed=${ELAPSED}s)" >&2
        CRASH_DETECTED=1
        tail -20 "$LOG_FILE" >&2
        echo "$TS,$ELAPSED,0,0,0,0,0,0,0,0,0,0" >> "$METRICS_FILE"
        break
    fi

    RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    RSS_MB=$((RSS_KB / 1024))
    FD_COUNT=$(lsof -p "$SERVER_PID" 2>/dev/null | wc -l | tr -d ' ')
    CPU_PCT=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    WAL_MB=0
    WAL_FILES=0
    if [ -d "$DATA_DIR" ]; then
        WAL_MB=$(du -sm "$DATA_DIR" 2>/dev/null | cut -f1 || echo 0)
        WAL_FILES=$(find "$DATA_DIR" -type f 2>/dev/null | wc -l | tr -d ' ')
    fi
    LOCK_COUNT=$(grep -c "^:" /proc/locks 2>/dev/null || echo 0)

    # Latest TPC-H round (count distinct dates in log)
    TPCH_ROUND=0
    if [ -f "$TPCH_LOG" ]; then
        TPCH_ROUND=$(awk -F, 'NR>1 {print $1}' "$TPCH_LOG" 2>/dev/null \
            | awk '{print substr($1,1,10)}' | sort -u | wc -l || echo 0)
    fi

    if [ "$SAMPLE_COUNT" -eq 1 ]; then
        INITIAL_RSS=$RSS_MB
        INITIAL_FD=$FD_COUNT
        INITIAL_WAL=$WAL_MB
    fi
    RSS_DELTA=$((RSS_MB - INITIAL_RSS))
    FD_DELTA=$((FD_COUNT - INITIAL_FD))

    echo "$TS,$ELAPSED,$RSS_MB,$RSS_DELTA,$FD_COUNT,$FD_DELTA,$CPU_PCT,$WAL_MB,$WAL_FILES,$LOCK_COUNT,1,$TPCH_ROUND" >> "$METRICS_FILE"

    if [ $((SAMPLE_COUNT % 12)) -eq 0 ]; then
        REMAIN_S=$((END_TS - $(date +%s)))
        REMAIN_M=$((REMAIN_S / 60))
        echo "  [${SAMPLE_COUNT} samples, ${ELAPSED}s elapsed, ${REMAIN_M}m remain] RSS=${RSS_MB}MB (d${RSS_DELTA}) FD=${FD_COUNT} (d${FD_DELTA}) CPU=${CPU_PCT}% WAL=${WAL_MB}MB TPC-H rounds=${TPCH_ROUND}"
    fi

    sleep "$INTERVAL"
done

echo ""
echo "=========================================="
echo "30-min TPC-H Wired Soak Complete"
echo "=========================================="

trap - EXIT
cleanup

FINAL_RSS=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f3)
FINAL_FD=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f5)
FINAL_WAL=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f8)
FINAL_RSS=${FINAL_RSS:-0}
FINAL_FD=${FINAL_FD:-0}
FINAL_WAL=${FINAL_WAL:-0}
RSS_GROWTH=$((FINAL_RSS - INITIAL_RSS))
FD_GROWTH=$((FINAL_FD - INITIAL_FD))
RSS_GROWTH=${RSS_GROWTH:-0}
FD_GROWTH=${FD_GROWTH:-0}

TPCH_ROUNDS=0
TPCH_ERRORS=0
if [ -f "$TPCH_LOG" ]; then
    TPCH_ROUNDS=$(awk -F, 'NR>1 {print $1}' "$TPCH_LOG" 2>/dev/null \
        | awk '{print substr($1,1,10)}' | sort -u | wc -l || echo 0)
    TPCH_ERRORS=$(grep -cE ",timeout_or_err" "$TPCH_LOG" 2>/dev/null || echo 0)
fi

CRASH_STATUS=$([ "$CRASH_DETECTED" -eq 0 ] && echo "Zero crashes" || echo "CRASH DETECTED")
RSS_OK=$([ "$RSS_GROWTH" -lt 50 ] && echo "PASS" || echo "WARN")
FD_OK=$([ "$FD_GROWTH" -lt 5 ] && echo "PASS" || echo "WARN")
TPCH_ROUNDS_OK=$([ "$TPCH_ROUNDS" -ge 25 ] && echo "PASS" || echo "WARN")

cat > "$RESULTS_DIR/STABILITY_REPORT.md" <<EOF
# 30-min TPC-H Wired Soak Report (sysbench-free)

**Mode**: SHORT_OBSERVATION (TPC-H only)
**Run timestamp**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Duration**: ${DURATION}s real wall-clock
**Workload**: TPC-H 22 query rotation (sysbench skipped — oltp_read_write.lua not in sysbench 1.0.20 install)
**Driver**: run_tpch_30min.sh (fallback for sysbench-minimal installs)

## Environment

| Item | Value |
|------|-------|
| Binary | $SQLRUSTGO_BIN |
| Host:Port | $HOST:$PORT |
| Data dir | $DATA_DIR |
| TPC-H fixture | $FIXTURE |
| TPC-H rotation | interval=${TPCH_INTERVAL}s, max_rounds=$TPCH_MAX_ROUNDS |

## Acceptance Results

| Criterion | Threshold | Measured | Status |
|-----------|-----------|----------|--------|
| Crashes | 0 | $CRASH_DETECTED | $CRASH_STATUS |
| RSS growth (30min) | < 50 MB | $RSS_GROWTH MB | $RSS_OK |
| FD growth (30min) | < 5 | $FD_GROWTH | $FD_OK |
| Final RSS | < 4096 MB | $FINAL_RSS MB | $([ "$FINAL_RSS" -lt 4096 ] && echo "PASS" || echo "WARN") |
| Final WAL | < 10240 MB | $FINAL_WAL MB | $([ "$FINAL_WAL" -lt 10240 ] && echo "PASS" || echo "WARN") |
| TPC-H rounds | ≥ 25 (out of 30 expected) | $TPCH_ROUNDS | $TPCH_ROUNDS_OK |
| TPC-H per-query errors | 0 ideal | $TPCH_ERRORS | $([ "$TPCH_ERRORS" -eq 0 ] && echo "PASS" || echo "WARN (Q2/Q9/Q13/Q16/Q17/Q20/Q21/Q22 known issues per PR-3262/3265)") |

## Verdict

$([ "$CRASH_DETECTED" -eq 0 ] && [ "$RSS_GROWTH" -lt 50 ] && echo "**PASS** — Pipeline complete, server stable under TPC-H 22 wired workload for 30 min" || echo "**NEEDS REVIEW** — See warnings above")
EOF

cat "$RESULTS_DIR/STABILITY_REPORT.md"
echo ""
echo "30-min TPC-H Soak Complete. Results: $RESULTS_DIR"
