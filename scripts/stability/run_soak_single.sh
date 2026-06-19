#!/bin/bash
# run_soak_single.sh - Single-step soak test (called by run_soak_ladder.sh)
# Runs: sqlrustgo-mysql-server + sysbench + resource guard for specified hours
#
# Env vars (must be passed from caller):
#   HOURS, THREADS, PORT, HOST, INTERVAL, SQLRUSTGO_BIN
#   RSS_HARD_LIMIT_MB, RSS_SOFT_MB, RSS_GROWTH_RATE_MB_PER_HR, DISK_MIN_GB, FD_HARD_LIMIT
#   RESULTS_DIR

set -euo pipefail

# Provide defaults for all env vars (SSH quoting can lose some)
HOURS="${HOURS:-1}"
THREADS="${THREADS:-8}"
PORT="${PORT:-3396}"
HOST="${HOST:-127.0.0.1}"
INTERVAL="${INTERVAL:-30}"
RESULTS_DIR="${RESULTS_DIR:-test_results/soak_default}"
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-./target/release/sqlrustgo-mysql-server}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Resource thresholds with defaults
RSS_HARD_LIMIT_MB="${RSS_HARD_LIMIT_MB:-8192}"
RSS_SOFT_MB="${RSS_SOFT_MB:-4096}"
RSS_GROWTH_RATE_MB_PER_HR="${RSS_GROWTH_RATE_MB_PER_HR:-200}"
DISK_MIN_GB="${DISK_MIN_GB:-2}"
FD_HARD_LIMIT="${FD_HARD_LIMIT:-8000}"

mkdir -p "$RESULTS_DIR"

DATA_DIR="$RESULTS_DIR/data"
PID_FILE="$RESULTS_DIR/sqlrustgo.pid"
LOG_FILE="$RESULTS_DIR/sqlrustgo.log"
SYSBENCH_LOG="$RESULTS_DIR/sysbench.log"
METRICS_FILE="$RESULTS_DIR/metrics.csv"
ALERT_LOG="$RESULTS_DIR/resource_alerts.log"

cleanup() {
    echo "[cleanup] Stopping resource monitor..."
    kill "$MONITOR_PID" 2>/dev/null || true
    wait "$MONITOR_PID" 2>/dev/null || true
    echo "[cleanup] Stopping sysbench..."
    kill "$SYSBENCH_PID" 2>/dev/null || true
    wait "$SYSBENCH_PID" 2>/dev/null || true
    echo "[cleanup] Stopping sqlrustgo server..."
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
    echo "[cleanup] Done"
}
trap cleanup EXIT

echo "=========================================="
echo "Soak Single Step: ${HOURS}h"
echo "=========================================="
echo "Port: $PORT  Threads: $THREADS  Binary: $SQLRUSTGO_BIN"
echo "Results: $RESULTS_DIR"
echo "RSS HARD: ${RSS_HARD_LIMIT_MB}MB  SOFT: ${RSS_SOFT_MB}MB"
echo "=========================================="

# ---- [1/4] Start server ----
if ! command -v sysbench >/dev/null 2>&1; then
    echo "FAIL: sysbench not found"
    exit 1
fi

nohup "$SQLRUSTGO_BIN" serve     --host "$HOST" --port "$PORT"     --data-dir "$DATA_DIR"     --log-level info     > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$PID_FILE"
echo "  Server PID: $SERVER_PID"

# Wait for ready
for i in $(seq 1 30); do
    sleep 1
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "FAIL: server died on startup"
        tail -20 "$LOG_FILE"
        exit 1
    fi
    if lsof -i ":$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
        echo "  Server ready after ${i}s"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "FAIL: server not listening after 30s"
        kill "$SERVER_PID" 2>/dev/null || true
        exit 1
    fi
done

# ---- [2/4] Start resource monitor ----
MONITOR_LOG="$RESULTS_DIR/resource_monitor.log" ALERT_LOG="$ALERT_LOG" INTERVAL="$INTERVAL" RSS_SOFT_MB="$RSS_SOFT_MB" RSS_HARD_LIMIT_MB="$RSS_HARD_LIMIT_MB" FD_HARD_LIMIT="$FD_HARD_LIMIT" DISK_MIN_GB="$DISK_MIN_GB" RSS_GROWTH_RATE_MB_PER_HR="$RSS_GROWTH_RATE_MB_PER_HR" bash "$SCRIPT_DIR/monitor_server.sh" "$PID_FILE" "$RESULTS_DIR" "$INTERVAL" > /dev/null 2>&1 &
MONITOR_PID=$!
echo "  Monitor PID: $MONITOR_PID"

# ---- [3/4] sysbench prepare + run ----
if ! sysbench oltp_read_write     --db-driver=mysql     --mysql-host="$HOST" --mysql-port="$PORT"     --mysql-user=root --mysql-password=""     --mysql-db=sbtest --table-size=10000 --tables=1     prepare 2>&1 | tail -3; then
    echo "FAIL: sysbench prepare failed"
    tail -20 "$LOG_FILE"
    exit 1
fi
echo "  sysbench prepare done"

nohup sysbench oltp_read_write --db-driver=mysql --db-ps-mode=disable --mysql-host="$HOST" --mysql-port="$PORT" --mysql-user=root --mysql-password="" --mysql-db=sbtest --table-size=10000 --tables=1 --threads="$THREADS" --time=$((HOURS*3600)) --report-interval=60 run > "$SYSBENCH_LOG" 2>&1 &
SYSBENCH_PID=$!
echo "  sysbench PID: $SYSBENCH_PID"

# ---- [4/4] Monitoring loop + resource guard ----
echo "ts,elapsed_s,rss_mb,rss_delta_mb,fd_count,fd_delta,cpu_pct,wal_mb,wal_files,server_alive,sysbench_alive" > "$METRICS_FILE"

START_TS=$(date +%s)
END_TS=$((START_TS + HOURS*3600))
INITIAL_RSS=0
INITIAL_FD=0
SAMPLE=0
CRASH=0

while [ "$(date +%s)" -lt "$END_TS" ]; do
    ELAPSED=$(($(date +%s) - START_TS))
    TS_STR=$(date '+%Y-%m-%dT%H:%M:%S')
    
    # Server alive?
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "  CRASH: server died at elapsed=${ELAPSED}s"
        CRASH=1
        break
    fi
    
    # Sysbench alive?
    SYSB_ALIVE=0
    kill -0 "$SYSBENCH_PID" 2>/dev/null && SYSB_ALIVE=1
    
    # Metrics
    RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    RSS_MB=$((RSS_KB / 1024))
    FD_COUNT=$(ls /proc/$SERVER_PID/fd 2>/dev/null | wc -l || echo 0)
    CPU_PCT=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    WAL_MB=$(du -sm "$DATA_DIR" 2>/dev/null | cut -f1 || echo 0)
    WAL_FILES=$(find "$DATA_DIR" -type f 2>/dev/null | wc -l || echo 0)
    
    if [ $SAMPLE -eq 0 ]; then
        INITIAL_RSS=$RSS_MB
        INITIAL_FD=$FD_COUNT
    fi
    RSS_DELTA=$((RSS_MB - INITIAL_RSS))
    FD_DELTA=$((FD_COUNT - INITIAL_FD))
    
    echo "$TS_STR,$ELAPSED,$RSS_MB,$RSS_DELTA,$FD_COUNT,$FD_DELTA,$CPU_PCT,$WAL_MB,$WAL_FILES,1,$SYSB_ALIVE" >> "$METRICS_FILE"
    
    # Resource guard — HARD limits (kill)
    if [ "$RSS_MB" -gt $RSS_HARD_LIMIT_MB ]; then
        echo "  KILL: RSS ${RSS_MB}MB > ${RSS_HARD_LIMIT_MB}MB"
        break
    fi
    if [ "$FD_COUNT" -gt $FD_HARD_LIMIT ]; then
        echo "  KILL: FD ${FD_COUNT} > ${FD_HARD_LIMIT}"
        break
    fi
    DISK_FREE_GB=$(df -BG /tmp 2>/dev/null | awk 'NR==2 {print $4}' | tr -d 'G')
    if [ "$DISK_FREE_GB" -lt $DISK_MIN_GB ]; then
        echo "  KILL: disk /tmp only ${DISK_FREE_GB}GB < ${DISK_MIN_GB}GB"
        break
    fi
    
    # Resource guard — SOFT warnings
    if [ "$RSS_MB" -gt $RSS_SOFT_MB ]; then
        echo "  WARN: RSS ${RSS_MB}MB > soft ${RSS_SOFT_MB}MB (delta=${RSS_DELTA}MB)"
    fi
    if [ $SAMPLE -gt 5 ]; then
        ELAPSED_H=$(awk "BEGIN {printf "%.2f", $ELAPSED / 3600}")
        if awk "BEGIN {exit !($ELAPSED_H > 0.05)}"; then
            :
        else
            GROWTH_RATE=$(awk "BEGIN {printf "%.0f", $RSS_DELTA / $ELAPSED_H}")
            if [ "$GROWTH_RATE" -gt $RSS_GROWTH_RATE_MB_PER_HR ]; then
                echo "  WARN: RSS growth rate ${GROWTH_RATE}MB/hr > ${RSS_GROWTH_RATE_MB_PER_HR}MB/hr"
            fi
        fi
    fi
    
    # Progress every 30 samples
    if [ $((SAMPLE % 30)) -eq 0 ] && [ $SAMPLE -gt 0 ]; then
        REMAIN_S=$((END_TS - $(date +%s)))
        REMAIN_H=$((REMAIN_S / 3600))
        echo "  [s${SAMPLE} ${ELAPSED}s ${REMAIN_H}h remain] RSS=${RSS_MB}MB(d${RSS_DELTA}) FD=${FD_COUNT}(d${FD_DELTA}) CPU=${CPU_PCT}% WAL=${WAL_MB}MB"
    fi
    
    SAMPLE=$((SAMPLE + 1))
    sleep "$INTERVAL"
done

trap - EXIT
cleanup

# ---- Generate report ----
FINAL_RSS=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f3 || echo 0)
FINAL_FD=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f5 || echo 0)
FINAL_WAL=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f8 || echo 0)
FINAL_CPU=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f7 || echo 0)
ACTUAL_ELAPSED=$(( $(date +%s) - START_TS))
ACTUAL_H=$(awk "BEGIN {printf "%.2f", $ACTUAL_ELAPSED / 3600}")
RSS_GROWTH=$((FINAL_RSS - INITIAL_RSS))
FD_GROWTH=$((FINAL_FD - INITIAL_FD))
SYSBENCH_TRANSACTIONS=$(grep -E "transactions:" "$SYSBENCH_LOG" 2>/dev/null | tail -1 || echo "TBD")
SYSBENCH_QPS=$(grep -E "queries per second" "$SYSBENCH_LOG" 2>/dev/null | tail -1 || echo "TBD")
SYSBENCH_ERRORS=$(grep -cE "FATAL|ERROR|deadlock" "$SYSBENCH_LOG" 2>/dev/null || echo 0)
ALERT_COUNT=$(wc -l < "$ALERT_LOG" 2>/dev/null || echo 0)
SAMPLE_COUNT=$(($(wc -l < "$METRICS_FILE") - 1))

if [ $CRASH -eq 1 ]; then
    VERDICT="FAIL — Server crashed"
elif [ $SYSBENCH_ERRORS -gt 0 ]; then
    VERDICT="WARN — sysbench errors"
elif [ $RSS_GROWTH -gt 100 ]; then
    VERDICT="PASS — ${ACTUAL_H}h, RSS grew ${RSS_GROWTH}MB (leak suspect)"
else
    VERDICT="PASS — ${ACTUAL_H}h soak, RSS +${RSS_GROWTH}MB, FD +${FD_GROWTH}"
fi

cat > "$RESULTS_DIR/step_${HOURS}h_REPORT.md" << EOF
# Soak Step: ${HOURS}h

**Duration**: ${ACTUAL_H}h (target ${HOURS}h)
**Binary**: $SQLRUSTGO_BIN
**Port**: $PORT

## Resource Results

| Criterion | Threshold | Measured | Status |
|-----------|-----------|----------|--------|
| Crash count | 0 | $CRASH | $([ $CRASH -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL") |
| RSS hard limit | ${RSS_HARD_LIMIT_MB}MB | ${FINAL_RSS}MB | $([ $FINAL_RSS -lt $RSS_HARD_LIMIT_MB ] && echo "✅ PASS" || echo "❌ FAIL") |
| FD hard limit | ${FD_HARD_LIMIT} | ${FINAL_FD} | $([ $FINAL_FD -lt $FD_HARD_LIMIT ] && echo "✅ PASS" || echo "❌ FAIL") |
| sysbench errors | 0 | $SYSBENCH_ERRORS | $([ $SYSBENCH_ERRORS -eq 0 ] && echo "✅ PASS" || echo "⚠️ WARN") |
| Alerts | 0 | $ALERT_COUNT | $([ $ALERT_COUNT -eq 0 ] && echo "✅ PASS" || echo "⚠️ WARN") |

## sysbench

$SYSBENCH_TRANSACTIONS
$SYSBENCH_QPS

## Metrics

- Samples: $SAMPLE_COUNT (every ${INTERVAL}s)
- Initial RSS: ${INITIAL_RSS}MB → Final: ${FINAL_RSS}MB (Δ ${RSS_GROWTH}MB)
- Initial FD: ${INITIAL_FD} → Final: ${FINAL_FD} (Δ ${FD_GROWTH})
- Final WAL: ${FINAL_WAL}MB

## Verdict

**$VERDICT**

$(if [ $CRASH -eq 0 ] && [ $SYSBENCH_ERRORS -eq 0 ]; then echo "✅ STEP PASS"; exit 0; else echo "❌ STEP FAIL"; exit 1; fi)
EOF

cat "$RESULTS_DIR/step_${HOURS}h_REPORT.md"
echo ""
echo "Step ${HOURS}h complete. Results: $RESULTS_DIR"
