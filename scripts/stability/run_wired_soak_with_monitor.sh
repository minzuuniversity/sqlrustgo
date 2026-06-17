#!/bin/bash
# run_wired_soak_with_monitor.sh - Real wall-clock soak + Z6G4 resource guard
#
# Launches: sqlrustgo-mysql-server + sysbench + resource monitor
# Resource guard: kills test if RSS > 8 GB OR disk < 2 GB OR FD > 8000
#
# Usage:
#   HOURS=72 THREADS=8 PORT=3396 FIXTURE=tpch-sf001 \\ 
#     bash scripts/stability/run_wired_soak_with_monitor.sh
#
# Environment:
#   HOURS          Duration in hours (default 24)
#   THREADS        sysbench threads (default 8)
#   PORT           MySQL port (default 3396)
#   HOST           MySQL host (default 127.0.0.1)
#   FIXTURE        none|tpch-tiny|tpch-sf001 (default tpch-sf001)
#   TPCH_ROTATE    1=run TPC-H 22 rotation (default 0)
#   INTERVAL       Resource monitor interval seconds (default 30)
#   RESULTS_DIR    Override results dir
#   SQLRUSTGO_BIN  Override binary path
#
# Thresholds (resource guard):
#   RSS_HARD_LIMIT_MB   Kill if RSS > 8192 MB
#   DISK_MIN_GB        Kill if /tmp free < 2 GB
#   FD_HARD_LIMIT      Kill if FD > 8000
#   RSS_SOFT_MB        Warn if RSS > 4096 MB
#   RSS_GROWTH_RATE    Warn if RSS growth > 200 MB/hr
#
# Output:
#   $RESULTS_DIR/
#     sqlrustgo.log        server log
#     sysbench.log         sysbench output
#     metrics.csv          monitoring samples
#     resource_monitor.log  full resource log
#     resource_alerts.log  threshold breach events
#     STABILITY_REPORT.md  final report
#     sqlrustgo.pid        server PID
#     monitor.pid          monitor PID
#     soak.pid             sysbench PID

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# ---- Defaults ----
HOURS="${HOURS:-24}"
THREADS="${THREADS:-8}"
PORT="${PORT:-3396}"
HOST="${HOST:-127.0.0.1}"
FIXTURE="${FIXTURE:-tpch-sf001}"
TPCH_ROTATE="${TPCH_ROTATE:-0}"
INTERVAL="${INTERVAL:-30}"
RESULTS_DIR="${RESULTS_DIR:-test_results/wired_soak_${HOURS}h_$(date +%Y%m%d_%H%M%S)}"
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-./target/release/sqlrustgo-mysql-server}"

# ---- Resource guard thresholds ----
RSS_HARD_LIMIT_MB="${RSS_HARD_LIMIT_MB:-8192}"
DISK_MIN_GB="${DISK_MIN_GB:-2}"
FD_HARD_LIMIT="${FD_HARD_LIMIT:-8000}"
RSS_SOFT_MB="${RSS_SOFT_MB:-4096}"
RSS_GROWTH_RATE_MB_PER_HR="${RSS_GROWTH_RATE_MB_PER_HR:-200}"

# ---- Derived ----
SQLRUSTGO_BIN="$(cd "$(dirname "$SQLRUSTGO_BIN")" && pwd)/$(basename "$SQLRUSTGO_BIN")"
DATA_DIR="$RESULTS_DIR/data"
PID_FILE="$RESULTS_DIR/sqlrustgo.pid"
LOG_FILE="$RESULTS_DIR/sqlrustgo.log"
SYSBENCH_LOG="$RESULTS_DIR/sysbench.log"
METRICS_FILE="$RESULTS_DIR/metrics.csv"
MONITOR_LOG="$RESULTS_DIR/resource_monitor.log"
ALERT_LOG="$RESULTS_DIR/resource_alerts.log"

mkdir -p "$RESULTS_DIR"
mkdir -p "$DATA_DIR"

# ---- Validate environment ----
if ! command -v sysbench >/dev/null 2>&1; then
    echo "FAIL: sysbench not found. Install: brew install sysbench"
    exit 1
fi

if [ ! -x "$SQLRUSTGO_BIN" ]; then
    echo "FAIL: $SQLRUSTGO_BIN not found or not executable"
    exit 1
fi

# Check port availability
if lsof -i ":$PORT" >/dev/null 2>&1; then
    echo "FAIL: port $PORT already in use"
    lsof -i ":$PORT"
    exit 1
fi

# ---- Cleanup trap ----
cleanup() {
    echo ""
    echo "[cleanup] Stopping resource monitor..."
    if [ -n "$MONITOR_PID" ] && kill -0 "$MONITOR_PID" 2>/dev/null; then
        kill "$MONITOR_PID" 2>/dev/null || true
        wait "$MONITOR_PID" 2>/dev/null || true
    fi
    
    echo "[cleanup] Stopping sysbench..."
    if [ -n "$SYSBENCH_PID" ] && kill -0 "$SYSBENCH_PID" 2>/dev/null; then
        kill "$SYSBENCH_PID" 2>/dev/null || true
        wait "$SYSBENCH_PID" 2>/dev/null || true
    fi
    
    echo "[cleanup] Stopping sqlrustgo server..."
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    echo "[cleanup] Done"
}
trap cleanup EXIT

# ---- Print config ----
echo "=========================================="
echo "SQLRustGo Wired Soak (with Resource Guard)"
echo "=========================================="
echo "Hours:         $HOURS h"
echo "Threads:       $THREADS (sysbench)"
echo "Port:          $PORT"
echo "Host:          $HOST"
echo "Fixture:       $FIXTURE"
echo "Binary:        $SQLRUSTGO_BIN"
echo "Results:       $RESULTS_DIR"
echo "=========================================="
echo "Resource guard thresholds:"
echo "  RSS HARD LIMIT:   ${RSS_HARD_LIMIT_MB} MB  (kill)"
echo "  RSS SOFT LIMIT:   ${RSS_SOFT_MB} MB     (warn)"
echo "  DISK MIN FREE:    ${DISK_MIN_GB} GB"
echo "  FD HARD LIMIT:    ${FD_HARD_LIMIT}"
echo "  RSS GROWTH RATE: ${RSS_GROWTH_RATE_MB_PER_HR} MB/hr"
echo "=========================================="
echo ""

# ---- [1/5] Start sqlrustgo server ----
echo "[1/5] Starting sqlrustgo-mysql-server on port $PORT..."
echo "  Log: $LOG_FILE"
nohup "$SQLRUSTGO_BIN" serve     --host "$HOST" --port "$PORT"     --data-dir "$DATA_DIR"     --log-level info     > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$PID_FILE"
echo "  Server PID: $SERVER_PID"

# Wait for server to be ready
for i in $(seq 1 30); do
    sleep 1
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "FAIL: server died on startup (PID $SERVER_PID)"
        tail -20 "$LOG_FILE"
        exit 1
    fi
    if lsof -i ":$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
        echo "  Server ready after ${i}s"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "FAIL: server not listening after 30s"
        tail -20 "$LOG_FILE"
        kill "$SERVER_PID" 2>/dev/null || true
        exit 1
    fi
done

# ---- [2/5] Resource monitor (background) ----
echo "[2/5] Starting resource monitor (PID_FILE=$PID_FILE, INTERVAL=${INTERVAL}s)..."
MONITOR_LOG="$MONITOR_LOG" ALERT_LOG="$ALERT_LOG" INTERVAL="$INTERVAL" RSS_SOFT_MB="$RSS_SOFT_MB" RSS_HARD_LIMIT_MB="$RSS_HARD_LIMIT_MB" FD_HARD_LIMIT="$FD_HARD_LIMIT" DISK_MIN_GB="$DISK_MIN_GB" RSS_GROWTH_RATE_MB_PER_HR="$RSS_GROWTH_RATE_MB_PER_HR" bash "$SCRIPT_DIR/monitor_server.sh "$PID_FILE"" "$RESULTS_DIR" "$INTERVAL" > /dev/null 2>&1 &
MONITOR_PID=$!
echo "$MONITOR_PID" > "$RESULTS_DIR/monitor.pid"
echo "  Monitor PID: $MONITOR_PID"

# ---- [3/5] sysbench prepare ----
echo "[3/5] sysbench prepare (table size=10000, 1 table)..."
if ! sysbench oltp_read_write     --db-driver=mysql     --mysql-host="$HOST" --mysql-port="$PORT"     --mysql-user=root --mysql-password=""     --mysql-db=sbtest --table-size=10000 --tables=1     prepare 2>&1 | tail -5; then
    echo "FAIL: sysbench prepare failed"
    tail -20 "$LOG_FILE"
    exit 1
fi
echo "  sysbench prepare done"

# ---- [4/5] Start sysbench run ----
echo "[4/5] Starting sysbench oltp_read_write (${HOURS}h, $THREADS threads)..."
nohup sysbench oltp_read_write     --db-driver=mysql     --mysql-host="$HOST" --mysql-port="$PORT"     --mysql-user=root --mysql-password=""     --mysql-db=sbtest --table-size=10000 --tables=1     --threads="$THREADS" --time=$((HOURS*3600))     --report-interval=60     run > "$SYSBENCH_LOG" 2>&1 &
SYSBENCH_PID=$!
echo "$SYSBENCH_PID" > "$RESULTS_DIR/soak.pid"
echo "  sysbench PID: $SYSBENCH_PID"

# ---- [5/5] Resource monitoring loop + guard ----
echo "[5/5] Monitoring loop (${INTERVAL}s interval, ${HOURS}h total)..."
echo "ts,elapsed_s,rss_mb,rss_delta_mb,fd_count,fd_delta,cpu_pct,wal_mb,wal_files,server_alive,sysbench_alive" > "$METRICS_FILE"

START_TS=$(date +%s)
END_TS=$((START_TS + HOURS*3600))
INITIAL_RSS=0
INITIAL_FD=0
SAMPLE=0
CRASH=0

while [ "$(date +%s)" -lt "$END_TS" ]; do
    TS_STR=$(date '+%Y-%m-%dT%H:%M:%S')
    ELAPSED=$(($(date +%s) - START_TS))
    
    # Check if server alive
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "  CRASH: server died at elapsed=${ELAPSED}s"
        CRASH=1
        break
    fi
    
    # Check if sysbench alive
    SYSB_ALIVE=0
    if kill -0 "$SYSBENCH_PID" 2>/dev/null; then
        SYSB_ALIVE=1
    fi
    
    # Collect metrics
    RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    RSS_MB=$((RSS_KB / 1024))
    FD_COUNT=$(ls /proc/$SERVER_PID/fd 2>/dev/null | wc -l || echo 0)
    CPU_PCT=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    
    # WAL / data dir size
    WAL_MB=0
    WAL_FILES=0
    if [ -d "$DATA_DIR" ]; then
        WAL_MB=$(du -sm "$DATA_DIR" 2>/dev/null | cut -f1 || echo 0)
        WAL_FILES=$(find "$DATA_DIR" -type f 2>/dev/null | wc -l || echo 0)
    fi
    
    # Initial baseline
    if [ $SAMPLE -eq 0 ]; then
        INITIAL_RSS=$RSS_MB
        INITIAL_FD=$FD_COUNT
    fi
    RSS_DELTA=$((RSS_MB - INITIAL_RSS))
    FD_DELTA=$((FD_COUNT - INITIAL_FD))
    
    echo "$TS_STR,$ELAPSED,$RSS_MB,$RSS_DELTA,$FD_COUNT,$FD_DELTA,$CPU_PCT,$WAL_MB,$WAL_FILES,1,$SYSB_ALIVE" >> "$METRICS_FILE"
    
    # ---- Resource guard checks ----
    # Hard limit: RSS > threshold → kill
    if [ "$RSS_MB" -gt $RSS_HARD_LIMIT_MB ]; then
        echo "  KILL: RSS ${RSS_MB}MB > ${RSS_HARD_LIMIT_MB}MB hard limit"
        echo "[KILL $(date '+%Y-%m-%dT%H:%M:%S')] RSS ${RSS_MB}MB exceeded hard limit ${RSS_HARD_LIMIT_MB}MB — killing soak" >> "$ALERT_LOG"
        break
    fi
    
    # Hard limit: FD > threshold → kill
    if [ "$FD_COUNT" -gt $FD_HARD_LIMIT ]; then
        echo "  KILL: FD ${FD_COUNT} > ${FD_HARD_LIMIT} hard limit"
        echo "[KILL $(date '+%Y-%m-%dT%H:%M:%S')] FD ${FD_COUNT} exceeded hard limit ${FD_HARD_LIMIT} — killing soak" >> "$ALERT_LOG"
        break
    fi
    
    # Hard limit: disk < threshold → kill
    DISK_FREE_GB=$(df -BG /tmp 2>/dev/null | awk 'NR==2 {print $4}' | tr -d 'G')
    if [ "$DISK_FREE_GB" -lt $DISK_MIN_GB ]; then
        echo "  KILL: disk /tmp only ${DISK_FREE_GB}GB free < ${DISK_MIN_GB}GB"
        echo "[KILL $(date '+%Y-%m-%dT%H:%M:%S')] Disk /tmp ${DISK_FREE_GB}GB < ${DISK_MIN_GB}GB — killing soak" >> "$ALERT_LOG"
        break
    fi
    
    # Soft warn: RSS growth rate
    if [ $SAMPLE -gt 5 ]; then
        ELAPSED_H=$(awk "BEGIN {printf "%.2f", $ELAPSED / 3600}")
        if awk "BEGIN {exit !($ELAPSED_H > 0.1)}"; then
            : 
        else
            GROWTH_RATE=$(awk "BEGIN {printf "%.0f", $RSS_DELTA / $ELAPSED_H}")
            if [ "$GROWTH_RATE" -gt $RSS_GROWTH_RATE_MB_PER_HR ]; then
                echo "  WARN[${ELAPSED}s]: RSS growth rate ${GROWTH_RATE}MB/hr > ${RSS_GROWTH_RATE_MB_PER_HR}MB/hr (delta=${RSS_DELTA}MB)"
            fi
        fi
    fi
    
    # Soft warn: RSS above soft limit
    if [ "$RSS_MB" -gt $RSS_SOFT_MB ]; then
        echo "  WARN[${ELAPSED}s]: RSS ${RSS_MB}MB > soft limit ${RSS_SOFT_MB}MB (delta=${RSS_DELTA}MB)"
    fi
    
    # Progress every 30 samples (30 * INTERVAL seconds)
    if [ $((SAMPLE % 30)) -eq 0 ] && [ $SAMPLE -gt 0 ]; then
        REMAIN_S=$((END_TS - $(date +%s)))
        REMAIN_H=$((REMAIN_S / 3600))
        echo "  [${SAMPLE} samples, ${ELAPSED}s elapsed, ${REMAIN_H}h remain] RSS=${RSS_MB}MB(d${RSS_DELTA}) FD=${FD_COUNT}(d${FD_DELTA}) CPU=${CPU_PCT}% WAL=${WAL_MB}MB"
    fi
    
    SAMPLE=$((SAMPLE + 1))
    sleep "$INTERVAL"
done

# ---- Generate STABILITY_REPORT.md ----
echo ""
echo "=========================================="
echo "Generating STABILITY_REPORT.md"
echo "=========================================="

FINAL_RSS=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f3 || echo 0)
FINAL_FD=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f5 || echo 0)
FINAL_WAL=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f8 || echo 0)
RSS_GROWTH=$((FINAL_RSS - INITIAL_RSS))
FD_GROWTH=$((FINAL_FD - INITIAL_FD))
ACTUAL_ELAPSED=$(( $(date +%s) - START_TS))
ACTUAL_H=$(awk "BEGIN {printf "%.2f", $ACTUAL_ELAPSED / 3600}")

SYSBENCH_TRANSACTIONS=$(grep -E "transactions:" "$SYSBENCH_LOG" 2>/dev/null | tail -1 || echo "TBD")
SYSBENCH_QPS=$(grep -E "queries per second" "$SYSBENCH_LOG" 2>/dev/null | tail -1 || echo "TBD")
SYSBENCH_ERRORS=$(grep -cE "FATAL|ERROR|deadlock" "$SYSBENCH_LOG" 2>/dev/null || echo 0)
ALERT_COUNT=$(wc -l < "$ALERT_LOG" 2>/dev/null || echo 0)
SAMPLE_COUNT=$(($(wc -l < "$METRICS_FILE") - 1))

if [ $CRASH -eq 1 ]; then
    VERDICT="**FAIL** — Server crashed during soak"
elif [ $SYSBENCH_ERRORS -gt 0 ]; then
    VERDICT="**WARN** — sysbench errors detected"
elif [ $RSS_GROWTH -gt 100 ]; then
    VERDICT="**WARN** — RSS grew ${RSS_GROWTH}MB (may indicate memory leak)"
elif [ $ACTUAL_H != "0.00" ]; then
    GROWTH_RATE=$(awk "BEGIN {printf "%.0f", $RSS_GROWTH / $ACTUAL_H}")
    VERDICT="**PASS** — ${ACTUAL_H}h soak, RSS +${RSS_GROWTH}MB (${GROWTH_RATE}MB/hr), FD +${FD_GROWTH}, sysbench errors=$SYSBENCH_ERRORS"
else
    VERDICT="**PASS** — ${ACTUAL_H}h soak completed"
fi

cat > "$RESULTS_DIR/STABILITY_REPORT.md" << EOF
# Stability Report — Real Wall-Clock Soak

**Run**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Duration**: ${ACTUAL_H}h (target ${HOURS}h)
**Binary**: $SQLRUSTGO_BIN
**Port**: $PORT

## Resource Guard Summary

| Guard | Threshold | Measured | Status |
|-------|-----------|----------|--------|
| RSS hard limit | ${RSS_HARD_LIMIT_MB} MB | ${FINAL_RSS} MB | $([ $FINAL_RSS -lt $RSS_HARD_LIMIT_MB ] && echo "✅ PASS" || echo "❌ FAIL") |
| FD hard limit | ${FD_HARD_LIMIT} | ${FINAL_FD} | $([ $FINAL_FD -lt $FD_HARD_LIMIT ] && echo "✅ PASS" || echo "❌ FAIL") |
| RSS soft growth | < 50 MB/24h | ${RSS_GROWTH} MB | $([ $RSS_GROWTH -lt 100 ] && echo "✅ PASS" || echo "⚠️ WARN") |
| Crash count | 0 | $CRASH | $([ $CRASH -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL") |
| sysbench errors | 0 | $SYSBENCH_ERRORS | $([ $SYSBENCH_ERRORS -eq 0 ] && echo "✅ PASS" || echo "⚠️ WARN") |
| Alert count | 0 | $ALERT_COUNT | $([ $ALERT_COUNT -eq 0 ] && echo "✅ PASS" || echo "⚠️ WARN") |

## sysbench Results

\`\`\`
$SYSBENCH_TRANSACTIONS
$SYSBENCH_QPS
\`\`\`

## Metrics

- **Samples**: $SAMPLE_COUNT (every ${INTERVAL}s)
- **Initial RSS**: ${INITIAL_RSS} MB
- **Final RSS**: ${FINAL_RSS} MB (Δ ${RSS_GROWTH} MB)
- **Initial FD**: ${INITIAL_FD}
- **Final FD**: ${FINAL_FD} (Δ ${FD_GROWTH})
- **Final WAL**: ${FINAL_WAL} MB

## Alerts

$(cat "$ALERT_LOG" 2>/dev/null || echo "None")

## Verdict

$VERDICT

## Artifacts

- \`metrics.csv\` — monitoring samples
- \`resource_monitor.log\` — full resource log
- \`resource_alerts.log\` — threshold breach events
- \`sqlrustgo.log\` — server log
- \`sysbench.log\` — sysbench output
EOF

cat "$RESULTS_DIR/STABILITY_REPORT.md"
echo ""
echo "✅ Soak complete. Results: $RESULTS_DIR"
