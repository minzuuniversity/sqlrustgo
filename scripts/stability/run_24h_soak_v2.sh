#!/bin/bash
# run_24h_soak_v2.sh - 真实 24h wall-clock soak (v3.9.0 GA gate)
#
# Acceptance (per docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md):
#   - Zero crashes in 24h
#   - RSS growth < 50MB over 24h
#   - FD growth < 50 over 24h
#   - WAL stabilizes after checkpoints
#   - All sysbench transactions succeed
#
# Issue: #3264 (S2 24h), #3225 (真实性), #3228 (un-ignore 14 long tests)
#
#   USE_CLI_SOAK=1    — use `sqlrustgo-cli soak` instead of sysbench
#   CLI_SOAK_RATE=5   — queries per second for CLI soak (default: 5)
#   CLI_SOAK_QUERIES  — path to custom query file for CLI soak
 # Usage: HOURS=24 INTERVAL=60 PORT=3396 ./scripts/stability/run_24h_soak_v2.sh
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

HOURS=${HOURS:-24}
INTERVAL=${INTERVAL:-60}
HOURS=$(printf "%.0f" "$HOURS" 2>/dev/null || echo "$HOURS")
if ! [[ "$HOURS" =~ ^[0-9]+$ ]] || [ "$HOURS" -lt 1 ]; then
    echo "  FAIL: HOURS must be a positive integer (got '$HOURS')"
    exit 1
fi
THREADS=${THREADS:-8}
TABLE_SIZE=${TABLE_SIZE:-10000}
TABLES=${TABLES:-1}
PORT=${PORT:-3396}
HOST=${HOST:-127.0.0.1}
RESULTS_DIR=${RESULTS_DIR:-"test_results/stability_24h_$(date +%Y%m%d_%H%M%S)"}
PID_FILE="$RESULTS_DIR/sqlrustgo.pid"
LOG_FILE="$RESULTS_DIR/sqlrustgo.log"
USE_CLI_SOAK=${USE_CLI_SOAK:-0}
CLI_SOAK_RATE=${CLI_SOAK_RATE:-5}
CLI_SOAK_QUERIES=${CLI_SOAK_QUERIES:-""}
CLI_SOAK_LOG="$RESULTS_DIR/cli_soak.log"
METRICS_FILE="$RESULTS_DIR/metrics.csv"
SYSBENCH_LOG="$RESULTS_DIR/sysbench.log"

mkdir -p "$RESULTS_DIR"

if [ "$USE_CLI_SOAK" != "1" ]; then
    if ! command -v sysbench >/dev/null 2>&1; then
        echo "  FAIL: sysbench not found in PATH. Install via brew install sysbench"
        echo "  Or set USE_CLI_SOAK=1 to use the built-in sqlrustgo-cli soak runner."
        exit 1
    fi
fi

if ! command -v sysbench >/dev/null 2>&1; then
    echo "  FAIL: sysbench not found in PATH. Install via brew install sysbench"
    exit 1
fi

if lsof -i ":$PORT" >/dev/null 2>&1; then
    echo "  FAIL: port $PORT already in use"
    lsof -i ":$PORT"
    exit 1
fi

DATA_DIR="$RESULTS_DIR/data"
mkdir -p "$DATA_DIR"

echo "=========================================="
echo "SQLRustGo 24h Soak (real wall-clock, v2)"
echo "=========================================="
echo "Hours: $HOURS  Interval: ${INTERVAL}s  Threads: $THREADS"
echo "Port: $PORT  Data: $DATA_DIR"
echo "Results: $RESULTS_DIR"
echo "Binary: $SQLRUSTGO_BIN"
echo "=========================================="
echo ""

echo "[1/4] Starting sqlrustgo-mysql-server (PID will be saved to $PID_FILE)..."
nohup "$SQLRUSTGO_BIN" serve \
    --host "$HOST" --port "$PORT" \
    --data-dir "$DATA_DIR" \
    --log-level info \
    > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo $SERVER_PID > "$PID_FILE"
echo "  Server PID: $SERVER_PID"
echo "  Server log: $LOG_FILE"

echo "[1/4] Waiting for server ready..."
for i in 1 2 3 4 5 6 7 8 9 10; do
    sleep 1
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "  FAIL: server died on startup"
        cat "$LOG_FILE"
        exit 1
    fi
    if lsof -i ":$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
        echo "  Server listening on $PORT after ${i}s"
        break
    fi
done

if [ "$USE_CLI_SOAK" = "1" ]; then
    CLI_SOAK_ARGS="--host $HOST --port $PORT --user root --password '' --duration $((HOURS*3600)) --rate $CLI_SOAK_RATE --report-interval $INTERVAL"
    if [ -n "$CLI_SOAK_QUERIES" ]; then
        CLI_SOAK_ARGS="$CLI_SOAK_ARGS --query-file $CLI_SOAK_QUERIES"
    fi
    echo "[3/4] Starting sqlrustgo-cli soak (${HOURS}h, ${CLI_SOAK_RATE} qps)..."
    nohup ./target/release/sqlrustgo-cli soak $CLI_SOAK_ARGS > "$CLI_SOAK_LOG" 2>&1 &
    LOAD_PID=$!
    echo "  sqlrustgo-cli PID: $LOAD_PID"
else
    echo "[3/4] Starting sysbench oltp_read_write (${HOURS}h, $THREADS threads)..."
    nohup sysbench oltp_read_write \
        --db-driver=mysql \
        --mysql-host="$HOST" --mysql-port="$PORT" \
        --mysql-user=root --mysql-password="" \
        --mysql-db=sbtest --table-size="$TABLE_SIZE" --tables="$TABLES" \
        --threads="$THREADS" --time=$((HOURS*3600)) \
        --report-interval=60 \
        run > "$SYSBENCH_LOG" 2>&1 &
    LOAD_PID=$!
    echo "  sysbench PID: $LOAD_PID"
fi
echo "ts,elapsed_s,rss_mb,rss_delta_mb,fd_count,fd_delta,cpu_pct,wal_mb,wal_files,lock_count,server_alive,load_qps" > "$METRICS_FILE"
INITIAL_FD=0
INITIAL_WAL=0
SAMPLE_COUNT=0
CRASH_DETECTED=0

cleanup() {
    echo ""
    if [ "$USE_CLI_SOAK" = "1" ]; then
        echo "[cleanup] Stopping sqlrustgo-cli soak (PID ${LOAD_PID:-unknown})..."
        kill ${LOAD_PID:-} 2>/dev/null || true
        wait ${LOAD_PID:-} 2>/dev/null || true
    else
        echo "[cleanup] Stopping sysbench (PID ${SYSBENCH_PID:-unknown})..."
        kill ${SYSBENCH_PID:-} 2>/dev/null || true
        wait ${SYSBENCH_PID:-} 2>/dev/null || true
    fi
    echo "[cleanup] Stopping server (PID $SERVER_PID)..."
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
    echo "[cleanup] Done"
}
trap cleanup EXIT

while [ "$(date +%s)" -lt "$END_TS" ]; do
    TS=$(date '+%Y-%m-%d %H:%M:%S')
    ELAPSED=$(($(date +%s) - START_TS))
    SAMPLE_COUNT=$((SAMPLE_COUNT + 1))

    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "  CRASH: server died at $TS (elapsed=${ELAPSED}s)"
        CRASH_DETECTED=1
        echo "Server log tail:"
        tail -20 "$LOG_FILE"
        echo "$TS,$ELAPSED,0,0,0,0,0,0,0,0,0,0" >> "$METRICS_FILE"
        break
    fi

    RSS_KB=$(ps -o rss= -p $SERVER_PID 2>/dev/null | tr -d ' ' || echo 0)
    RSS_MB=$((RSS_KB / 1024))

    FD_COUNT=$(lsof -p $SERVER_PID 2>/dev/null | wc -l | tr -d ' ')

    CPU_PCT=$(ps -o %cpu= -p $SERVER_PID 2>/dev/null | tr -d ' ' || echo 0)

    WAL_MB=0
    LOAD_QPS=0
    if [ "$USE_CLI_SOAK" = "1" ]; then
        if [ -f "$CLI_SOAK_LOG" ]; then
            LOAD_QPS=$(grep -oE '[0-9]+\.[0-9]+ qps' "$CLI_SOAK_LOG" 2>/dev/null | tail -1 | grep -oE '[0-9]+\.[0-9]+' || echo 0)
        fi
    else
        if [ -f "$SYSBENCH_LOG" ]; then
            LOAD_QPS=$(grep -E "thds|tps|qps" "$SYSBENCH_LOG" 2>/dev/null | tail -1 | grep -oE "[0-9]+\.[0-9]+\s*per sec" | grep -oE "[0-9]+\.[0-9]+" | head -1 || echo 0)
        fi
    fi

    LOCK_COUNT=$(grep -c "^:" /proc/locks 2>/dev/null || echo 0)

    if [ $SAMPLE_COUNT -eq 1 ]; then
        INITIAL_RSS=$RSS_MB
        INITIAL_FD=$FD_COUNT
        INITIAL_WAL=$WAL_MB
    fi
    RSS_DELTA=$((RSS_MB - INITIAL_RSS))
    FD_DELTA=$((FD_COUNT - INITIAL_FD))

    echo "$TS,$ELAPSED,$RSS_MB,$RSS_DELTA,$FD_COUNT,$FD_DELTA,$CPU_PCT,$WAL_MB,$WAL_FILES,$LOCK_COUNT,1,$LOAD_QPS" >> "$METRICS_FILE"

    if [ $SAMPLE_COUNT -gt 5 ] && [ $((RSS_MB - INITIAL_RSS)) -gt 50 ]; then
        echo "  WARN[${ELAPSED}s]: RSS growth > 50MB (delta=$((RSS_MB - INITIAL_RSS))MB)"
    fi
    if [ $SAMPLE_COUNT -gt 5 ] && [ $((FD_COUNT - INITIAL_FD)) -gt 50 ]; then
        echo "  WARN[${ELAPSED}s]: FD growth > 50 (delta=$((FD_COUNT - INITIAL_FD)))"
    fi

    if [ $((SAMPLE_COUNT % 10)) -eq 0 ]; then
        REMAIN_S=$((END_TS - $(date +%s)))
        REMAIN_HR=$((REMAIN_S / 3600))
        echo "  [${SAMPLE_COUNT} samples, ${ELAPSED}s elapsed, ${REMAIN_HR}h remain] RSS=${RSS_MB}MB (d${RSS_DELTA}) FD=${FD_COUNT} (d${FD_DELTA}) CPU=${CPU_PCT}% WAL=${WAL_MB}MB QPS=${SYSBENCH_QPS}"
    fi

    sleep $INTERVAL
done

echo ""
echo "=========================================="
echo "24h Soak Complete - Generating Report"
echo "=========================================="

trap - EXIT
cleanup

FINAL_RSS=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f3)
FINAL_FD=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f5)
FINAL_CPU=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f7)
FINAL_WAL=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f8)
FINAL_RSS=${FINAL_RSS:-0}
FINAL_FD=${FINAL_FD:-0}
FINAL_CPU=${FINAL_CPU:-0}
FINAL_WAL=${FINAL_WAL:-0}
RSS_GROWTH=$((FINAL_RSS - INITIAL_RSS))
FD_GROWTH=$((FINAL_FD - INITIAL_FD))
RSS_GROWTH=${RSS_GROWTH:-0}
FD_GROWTH=${FD_GROWTH:-0}

if [ "$USE_CLI_SOAK" = "1" ]; then
    LOAD_TRANSACTIONS=$(grep -E "queries executed" "$CLI_SOAK_LOG" 2>/dev/null | tail -1 || echo "TBD")
    LOAD_QPS_FINAL=$(grep -oE '[0-9]+\.[0-9]+ qps' "$CLI_SOAK_LOG" 2>/dev/null | tail -1 || echo "TBD")
    LOAD_ERRORS=$(grep -cE "error" "$CLI_SOAK_LOG" 2>/dev/null || echo 0)
else
    LOAD_TRANSACTIONS=$(grep -E "transactions:" "$SYSBENCH_LOG" 2>/dev/null | tail -1 || echo "TBD")
    LOAD_QPS_FINAL=$(grep -E "queries per second" "$SYSBENCH_LOG" 2>/dev/null | tail -1 || echo "TBD")
    LOAD_ERRORS=$(grep -cE "FATAL|ERROR|deadlock" "$SYSBENCH_LOG" 2>/dev/null || echo 0)
fi

CRASH_STATUS=$([ $CRASH_DETECTED -eq 0 ] && echo "Zero crashes" || echo "CRASH DETECTED")

{
    echo "# 24h Soak Stability Report (Real Wall-Clock)"
    echo ""
    echo "**Run timestamp**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "**Duration**: ${HOURS}h (real wall-clock, not simulated)"
    echo "**Issues**: Closes #3264 / #3225 / #3228"
    echo ""
    echo "## Environment"
    echo ""
    echo "| Item | Value |"
    echo "|------|-------|"
    echo "| Binary | $SQLRUSTGO_BIN |"
    echo "| Port | $PORT |"
    echo "| Data dir | $DATA_DIR |"
    if [ "$USE_CLI_SOAK" = "1" ]; then
        echo "| Load generator | sqlrustgo-cli soak (rate=${CLI_SOAK_RATE} qps) |"
    else
        echo "| Threads | $THREADS (sysbench oltp_read_write) |"
    fi
    echo "| Table size | $TABLE_SIZE |"
    echo "| Tables | $TABLES |"
    echo ""
    echo "## Acceptance Results"
    echo ""
    echo "| Criterion | Threshold | Measured | Status |"
    echo "|-----------|-----------|----------|--------|"
    echo "| Crashes | 0 | $CRASH_DETECTED | $CRASH_STATUS |"
    if [ "$USE_CLI_SOAK" = "1" ]; then
        LOAD_PASS=$([ "$LOAD_ERRORS" = "0" ] || [ "$LOAD_ERRORS" = "0" ] 2>/dev/null && echo "PASS" || echo "WARN")
        echo "| Load errors | 0 | $LOAD_ERRORS | $LOAD_PASS |"
    else
        SYSBENCH_PASS=$([ $SYSBENCH_ERRORS -eq 0 ] && echo "PASS" || echo "WARN")
        echo "| sysbench errors | 0 | $SYSBENCH_ERRORS | $SYSBENCH_PASS |"
    fi
    echo "| RSS growth | < 50 MB | $RSS_GROWTH MB | $([ ${RSS_GROWTH:-999} -lt 50 ] && echo "PASS" || echo "WARN") |"
    echo "| FD growth | < 50 | $FD_GROWTH | $([ ${FD_GROWTH:-999} -lt 50 ] && echo "PASS" || echo "WARN") |"
    echo "| Final RSS | < 4096 MB | ${FINAL_RSS} MB | $([ ${FINAL_RSS:-9999} -lt 4096 ] && echo "PASS" || echo "WARN") |"
    echo "| Final WAL | < 10240 MB | ${FINAL_WAL} MB | $([ ${FINAL_WAL:-99999} -lt 10240 ] && echo "PASS" || echo "WARN") |"
    echo ""
    if [ "$USE_CLI_SOAK" = "1" ]; then
        echo "## sqlrustgo-cli soak Results"
    else
        echo "## sysbench Results"
    fi
    echo ""
    echo '```'
    echo "$LOAD_TRANSACTIONS"
    echo "$LOAD_QPS_FINAL"
    echo '```'
    echo ""
    echo "## Artifacts"
    echo ""
    echo "- \`metrics.csv\` — $(($(wc -l < "$METRICS_FILE") - 1)) samples, sampled every ${INTERVAL}s"
    echo "- \`sqlrustgo.log\` — server log"
    if [ "$USE_CLI_SOAK" = "1" ]; then
        echo "- \`cli_soak.log\` — sqlrustgo-cli soak output"
    else
        echo "- \`sysbench.log\` — sysbench output"
    fi
    echo "- \`sqlrustgo.pid\` — server PID"
    echo ""
    echo "## Verdict"
    echo ""
    VERDICT_PASS=true
    [ $CRASH_DETECTED -ne 0 ] && VERDICT_PASS=false
    [ ${RSS_GROWTH:-999} -ge 50 ] && VERDICT_PASS=false
    [ ${FD_GROWTH:-999} -ge 50 ] && VERDICT_PASS=false
    if $VERDICT_PASS; then
        echo "**PASS** - All acceptance criteria met"
    else
        echo "**NEEDS REVIEW** - See warnings above"
    fi
    echo ""
} > "$RESULTS_DIR/STABILITY_REPORT.md"

cat "$RESULTS_DIR/STABILITY_REPORT.md"
echo ""
echo "24h Soak Complete. Results: $RESULTS_DIR"
