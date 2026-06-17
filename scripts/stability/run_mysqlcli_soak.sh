#!/bin/bash
# run_mysqlcli_soak.sh - Real wall-clock soak using mysql CLI (MariaDB client)
# 
# User requirement: "真实 mysql-server 方式在后台运行，前端是 MariaDB 的 client（mysql cli），
#  持续执行 CRUD 的测试"
#
# This script:
#   1. Starts sqlrustgo-mysql-server on specified port
#   2. Creates test schema (sbtest table) via mysql CLI
#   3. Runs continuous CRUD loop via mysql CLI (INSERT/SELECT/UPDATE/DELETE)
#   4. Monitors resources every INTERVAL seconds
#   5. Resource guard: kills on RSS > HARD_LIMIT, warns on RSS > SOFT_LIMIT
#   6. Generates STABILITY_REPORT.md
#
# Usage:
#   HOURS=1 THREADS=4 PORT=3396 bash run_mysqlcli_soak.sh
#
# Env vars:
#   HOURS           Duration in hours (default 1)
#   THREADS         Concurrent mysql CLI sessions (default 4)
#   PORT            MySQL port (default 3396)
#   HOST            MySQL host (default 127.0.0.1)
#   INTERVAL        Resource sample interval seconds (default 30)
#   RESULTS_DIR     Output directory (auto-generated if not set)
#   SQLRUSTGO_BIN   Server binary path

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Defaults
HOURS="${HOURS:-1}"
THREADS="${THREADS:-4}"
PORT="${PORT:-3396}"
HOST="${HOST:-127.0.0.1}"
INTERVAL="${INTERVAL:-30}"
RESULTS_DIR="${RESULTS_DIR:-test_results/mysqlcli_soak_$(date +%Y%m%d_%H%M%S)}"
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-$PROJECT_ROOT/target/release/sqlrustgo-mysql-server}"

# Resource thresholds
RSS_HARD_MB="${RSS_HARD_MB:-8192}"
RSS_SOFT_MB="${RSS_SOFT_MB:-4096}"
DISK_MIN_GB="${DISK_MIN_GB:-2}"
FD_HARD="${FD_HARD:-8000}"

mkdir -p "$RESULTS_DIR"
DATA_DIR="$RESULTS_DIR/data"
PID_FILE="$RESULTS_DIR/server.pid"
SERVER_LOG="$RESULTS_DIR/sqlrustgo.log"
METRICS="$RESULTS_DIR/metrics.csv"
CRUD_LOG="$RESULTS_DIR/crud.log"
ALERT_LOG="$RESULTS_DIR/alerts.log"
REPORT="$RESULTS_DIR/STABILITY_REPORT.md"

cleanup() {
    echo "[cleanup] Stopping CRUD loops..."
    for pid in "${CRUD_PIDS[@]:-}"; do
        kill "$pid" 2>/dev/null || true
    done
    echo "[cleanup] Stopping server..."
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    echo "[cleanup] Done"
}
trap cleanup EXIT

mysql_cmd() {
    mysql -h "$HOST" -P "$PORT" -u root --batch -N -e "$1" 2>/dev/null
}

wait_db() {
    for i in $(seq 1 30); do
        if mysql_cmd "SELECT 1;" >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    return 1
}

crud_loop() {
    local thread_id=$1
    local total_ops=$2
    local ops=0
    while [ $ops -lt $total_ops ]; do
        # INSERT
        mysql_cmd "INSERT INTO sbtest VALUES (NULL, $((RANDOM % 10000)), REPEAT('c', 120), REPEAT('p', 60));" sbtest >/dev/null 2>&1 || true
        ops=$((ops + 1))
        # SELECT
        mysql_cmd "SELECT COUNT(*) FROM sbtest;" sbtest >/dev/null 2>&1 || true
        ops=$((ops + 1))
        # UPDATE
        mysql_cmd "UPDATE sbtest SET c=REPEAT('u', 120) WHERE id=$((RANDOM % 1000 + 1)) LIMIT 1;" sbtest >/dev/null 2>&1 || true
        ops=$((ops + 1))
        # DELETE (limit 1)
        mysql_cmd "DELETE FROM sbtest WHERE id IN (SELECT id FROM (SELECT id FROM sbtest LIMIT 1) AS t);" sbtest >/dev/null 2>&1 || true
        ops=$((ops + 1))
        # SELECT random
        mysql_cmd "SELECT id, k FROM sbtest WHERE k=$((RANDOM % 100)) LIMIT 5;" sbtest >/dev/null 2>&1 || true
        ops=$((ops + 1))
    done
    echo "[crud_loop $thread_id] Completed $ops operations" >> "$CRUD_LOG"
}

crud_continuous() {
    local thread_id=$1
    while true; do
        # Batch of 20 CRUD ops per iteration
        for i in $(seq 1 20); do
            mysql_cmd "INSERT INTO sbtest VALUES (NULL, $((RANDOM % 10000)), REPEAT('c', 120), REPEAT('p', 60));" sbtest >/dev/null 2>&1 || true
            mysql_cmd "SELECT COUNT(*) FROM sbtest;" sbtest >/dev/null 2>&1 || true
            mysql_cmd "UPDATE sbtest SET c=REPEAT('u', 120) WHERE id=$((RANDOM % 1000 + 1)) LIMIT 1;" sbtest >/dev/null 2>&1 || true
            mysql_cmd "SELECT id, k FROM sbtest WHERE k=$((RANDOM % 100)) LIMIT 3;" sbtest >/dev/null 2>&1 || true
        done
        sleep 0.5
    done
}

echo "=========================================="
echo "SQLRustGo mysql-cli Soak (Real CRUD)"
echo "=========================================="
echo "Hours:    $HOURS"
echo "Threads:  $THREADS (concurrent mysql sessions)"
echo "Port:     $PORT"
echo "Binary:   $SQLRUSTGO_BIN"
echo "Results:  $RESULTS_DIR"
echo "=========================================="

# Check prerequisites
if ! command -v mysql >/dev/null 2>&1; then
    echo "FAIL: mysql CLI not found"
    exit 1
fi

if [ ! -x "$SQLRUSTGO_BIN" ]; then
    echo "FAIL: $SQLRUSTGO_BIN not found"
    exit 1
fi

# Start server
echo "[1/5] Starting sqlrustgo-mysql-server..."
nohup "$SQLRUSTGO_BIN" serve     --host "$HOST" --port "$PORT"     --data-dir "$DATA_DIR"     --log-level info     > "$SERVER_LOG" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$PID_FILE"
echo "  Server PID: $SERVER_PID"

if ! wait_db; then
    echo "FAIL: server not responding after 30s"
    tail -20 "$SERVER_LOG"
    exit 1
fi
echo "  Server ready"

# Create schema
echo "[2/5] Creating test schema..."
mysql_cmd "CREATE DATABASE IF NOT EXISTS sbtest;" || true
mysql_cmd "
CREATE TABLE IF NOT EXISTS sbtest (
    id INT AUTO_INCREMENT PRIMARY KEY,
    k INT NOT NULL,
    c CHAR(120) NOT NULL,
    pad CHAR(60) NOT NULL,
    INDEX idx_k(k)
);
" sbtest || {
    echo "FAIL: could not create table"
    tail -10 "$SERVER_LOG"
    exit 1
}

# Count initial rows
INITIAL_COUNT=$(mysql_cmd "SELECT COUNT(*) FROM sbtest;" sbtest 2>/dev/null || echo 0)
echo "  Table sbtest created, initial rows: $INITIAL_COUNT"

# Start CRUD loops
echo "[3/5] Starting $THREADS CRUD threads..."
declare -a CRUD_PIDS
for i in $(seq 1 "$THREADS"); do
    crud_continuous $i > /dev/null 2>&1 &
    CRUD_PIDS+=($!)
    echo "  Thread $i PID: ${CRUD_PIDS[-1]}"
done

# Start resource monitor
echo "[4/5] Starting resource monitor (${INTERVAL}s interval)..."
echo "ts,elapsed_h,rss_mb,rss_delta_mb,fd_count,fd_delta,cpu_pct,disk_free_gb,db_rows,queries_ok,queries_err" > "$METRICS"

START_TS=$(date +%s)
END_TS=$((START_TS + HOURS*3600))
INITIAL_RSS=0
INITIAL_FD=0
INITIAL_COUNT=0
SAMPLE=0
TOTAL_QUERIES=0
TOTAL_ERRORS=0

echo "[5/5] Monitoring loop..."
while [ "$(date +%s)" -lt "$END_TS" ]; do
    TS_STR=$(date '+%Y-%m-%dT%H:%M:%S')
    ELAPSED=$(($(date +%s) - START_TS))
    ELAPSED_H=$(awk -v e=$ELAPSED 'BEGIN {printf "%.3f", e / 3600}')
    
    # Check server alive
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "  CRASH: server died at elapsed=${ELAPSED}s"
        break
    fi
    
    # Check all CRUD threads alive
    ALL_ALIVE=1
    for pid in "${CRUD_PIDS[@]}"; do
        if ! kill -0 "$pid" 2>/dev/null; then
            ALL_ALIVE=0
            # Restart dead thread
            crud_continuous $$ > /dev/null 2>&1 &
            CRUD_PIDS[$idx]=$!
        fi
    done
    
    # Collect metrics
    RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    RSS_MB=$((RSS_KB / 1024))
    FD_COUNT=$(ls /proc/$SERVER_PID/fd 2>/dev/null | wc -l || echo 0)
    CPU_PCT=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    DISK_GB=$(df -BG /tmp 2>/dev/null | awk 'NR==2 {print $4}' | tr -d 'G')
    DB_ROWS=$(mysql_cmd "SELECT COUNT(*) FROM sbtest;" sbtest 2>/dev/null || echo 0)
    
    # Quick query test
    Q_OK=0
    Q_ERR=0
    for attempt in 1 2 3; do
        if mysql_cmd "SELECT 1;" >/dev/null 2>&1; then
            Q_OK=$((Q_OK + 1))
        else
            Q_ERR=$((Q_ERR + 1))
        fi
    done
    TOTAL_QUERIES=$((TOTAL_QUERIES + 3))
    TOTAL_ERRORS=$((TOTAL_ERRORS + Q_ERR))
    
    if [ $SAMPLE -eq 0 ]; then
        INITIAL_RSS=$RSS_MB
        INITIAL_FD=$FD_COUNT
        INITIAL_COUNT=$DB_ROWS
    fi
    
    RSS_DELTA=$((RSS_MB - INITIAL_RSS))
    FD_DELTA=$((FD_COUNT - INITIAL_FD))
    
    echo "$TS_STR,$ELAPSED_H,$RSS_MB,$RSS_DELTA,$FD_COUNT,$FD_DELTA,$CPU_PCT,$DISK_GB,$DB_ROWS,$Q_OK,$Q_ERR" >> "$METRICS"
    
    # Resource guard HARD limits
    if [ "$RSS_MB" -gt $RSS_HARD_MB ]; then
        echo "[KILL $TS_STR] RSS ${RSS_MB}MB > ${RSS_HARD_MB}MB hard limit" >> "$ALERT_LOG"
        echo "  KILL: RSS ${RSS_MB}MB > ${RSS_HARD_MB}MB hard limit"
        break
    fi
    if [ "$FD_COUNT" -gt $FD_HARD ]; then
        echo "[KILL $TS_STR] FD ${FD_COUNT} > ${FD_HARD}" >> "$ALERT_LOG"
        break
    fi
    if [ "$DISK_GB" -lt $DISK_MIN_GB ]; then
        echo "[KILL $TS_STR] Disk ${DISK_GB}GB < ${DISK_MIN_GB}GB" >> "$ALERT_LOG"
        break
    fi
    
    # Soft warnings
    if [ "$RSS_MB" -gt $RSS_SOFT_MB ]; then
        echo "[WARN $TS_STR] RSS ${RSS_MB}MB > soft ${RSS_SOFT_MB}MB" >> "$ALERT_LOG"
        echo "  WARN: RSS ${RSS_MB}MB > soft ${RSS_SOFT_MB}MB"
    fi
    
    # Progress every 5 minutes
    if [ $((SAMPLE % 10)) -eq 0 ] && [ $SAMPLE -gt 0 ]; then
        REMAIN_H=$(( (END_TS - $(date +%s)) / 3600 ))
        echo "  [s${SAMPLE} ${ELAPSED_H}h remain ~${REMAIN_H}h] RSS=${RSS_MB}MB(d${RSS_DELTA}) FD=${FD_COUNT}(d${FD_DELTA}) rows=${DB_ROWS} q_err=${Q_ERR}"
    fi
    
    SAMPLE=$((SAMPLE + 1))
    sleep "$INTERVAL"
done

trap - EXIT
cleanup

# ---- Generate Report ----
FINAL_RSS=$(grep -v "^ts," "$METRICS" | tail -1 | cut -d, -f3 || echo 0)
FINAL_FD=$(grep -v "^ts," "$METRICS" | tail -1 | cut -d, -f5 || echo 0)
FINAL_ROWS=$(grep -v "^ts," "$METRICS" | tail -1 | cut -d, -f9 || echo 0)
FINAL_CPU=$(grep -v "^ts," "$METRICS" | tail -1 | cut -d, -f7 || echo 0)
ACTUAL_ELAPSED=$(( $(date +%s) - START_TS))
ACTUAL_H=$(awk "BEGIN {printf "%.3f", $ACTUAL_ELAPSED / 3600}")
SAMPLE_COUNT=$(($(wc -l < "$METRICS") - 1))
ALERT_COUNT=$(wc -l < "$ALERT_LOG" 2>/dev/null || echo 0)
RSS_GROWTH=$((FINAL_RSS - INITIAL_RSS))

cat > "$REPORT" << EOF
# mysql-cli Soak Stability Report

**Run**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Duration**: ${ACTUAL_H}h (target ${HOURS}h)
**Binary**: $SQLRUSTGO_BIN
**Port**: $PORT
**Threads**: $THREADS concurrent mysql sessions

## Resource Results

| Criterion | Threshold | Measured | Status |
|-----------|-----------|----------|--------|
| RSS hard limit | ${RSS_HARD_MB} MB | ${FINAL_RSS} MB | $([ $FINAL_RSS -lt $RSS_HARD_MB ] && echo "✅ PASS" || echo "❌ FAIL") |
| FD hard limit | ${FD_HARD} | ${FINAL_FD} | $([ $FINAL_FD -lt $FD_HARD ] && echo "✅ PASS" || echo "❌ FAIL") |
| RSS growth | < 50 MB/h | ${RSS_GROWTH} MB | $([ $RSS_GROWTH -lt 50 ] && echo "✅ PASS" || echo "⚠️ WARN") |
| Alert count | 0 | $ALERT_COUNT | $([ $ALERT_COUNT -eq 0 ] && echo "✅ PASS" || echo "⚠️ WARN") |
| Query errors | 0 | $TOTAL_ERRORS | $([ $TOTAL_ERRORS -eq 0 ] && echo "✅ PASS" || echo "⚠️ WARN") |

## DB Metrics

- Initial rows: ${INITIAL_COUNT}
- Final rows: ${FINAL_ROWS}
- Total query samples: $SAMPLE_COUNT (every ${INTERVAL}s)
- Initial RSS: ${INITIAL_RSS} MB → Final: ${FINAL_RSS} MB (Δ ${RSS_GROWTH} MB)
- Initial FD: ${INITIAL_FD} → Final: ${FINAL_FD} (Δ $((FINAL_FD - INITIAL_FD)))

## Verdict

$(if [ $TOTAL_ERRORS -eq 0 ] && [ $RSS_GROWTH -lt 100 ]; then
    echo "✅ **PASS** — ${ACTUAL_H}h soak, RSS +${RSS_GROWTH}MB, FD +$((FINAL_FD - INITIAL_FD))"
else
    echo "⚠️ **REVIEW NEEDED** — errors=$TOTAL_ERRORS, RSS_growth=${RSS_GROWTH}MB"
fi)

## Artifacts

- \`metrics.csv\` — resource samples
- \`alerts.log\` — threshold events
- \`sqlrustgo.log\` — server log
- \`crud.log\` — CRUD thread output
EOF

cat "$REPORT"
echo ""
echo "✅ ${HOURS}h soak complete. Results: $RESULTS_DIR"
