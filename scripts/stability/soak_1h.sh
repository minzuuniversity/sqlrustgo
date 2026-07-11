#!/bin/bash
# soak_1h.sh - 1h soak test using sqlrustgo-cli soak
# 24 threads, 40k table, realistic OLTP workload via bash-piped CLI
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Config
HOURS=${HOURS:-1}
THREADS=${THREADS:-24}
TABLE_SIZE=${TABLE_SIZE:-40000}
PORT=${PORT:-3307}
HOST=${HOST:-127.0.0.1}
RESULTS_DIR=${RESULTS_DIR:-"test_results/soak_1h_$(date +%Y%m%d_%H%M%S)"}
INTERVAL=${INTERVAL:-30}
CLI_BIN="${CLI_BIN:-./target/release/sqlrustgo-cli}"

mkdir -p "$RESULTS_DIR"
METRICS_FILE="$RESULTS_DIR/metrics.csv"
REPORT_FILE="$RESULTS_DIR/STABILITY_REPORT.md"
SOAK_LOG="$RESULTS_DIR/soak.log"
SERVER_LOG="$RESULTS_DIR/server.log"

echo "=========================================="
echo "SQLRustGo 1h Soak Test (sqlrustgo-cli soak)"
echo "=========================================="
echo "Hours: $HOURS"
echo "Threads: $THREADS"
echo "Table size: $TABLE_SIZE"
echo "Port: $PORT"
echo "Host: $HOST"
echo "Results: $RESULTS_DIR"
echo "=========================================="

# Find server PID
find_server_pid() {
    pgrep -f "sqlrustgo-mysql-server.*serve" 2>/dev/null | grep -v defunct | head -1 || echo ""
}

get_metrics() {
    local pid=$1
    if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
        echo "0 0"
        return
    fi
    local rss_kb=0 fd_count=0
    rss_kb=$(grep VmRSS /proc/$pid/status 2>/dev/null | awk '{print $2}' || echo 0)
    if [ -d "/proc/$pid/fd" ]; then
        fd_count=$(ls /proc/$pid/fd 2>/dev/null | wc -l || echo 0)
    fi
    echo "$rss_kb $fd_count"
}

# Check / start server
SERVER_PID=$(find_server_pid)
if [ -z "$SERVER_PID" ]; then
    echo "[1/5] Starting sqlrustgo server on port $PORT..."
    DATA_DIR="${DATA_DIR:-/tmp/sqlrustgo-soak-data}"
    mkdir -p "$DATA_DIR"
    nohup "$CLI_BIN" serve --port "$PORT" --data-dir "$DATA_DIR" > "$SERVER_LOG" 2>&1 &
    SERVER_PID=$!
    echo "  Server PID: $SERVER_PID"
    sleep 5
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "ERROR: Server failed to start"
        cat "$SERVER_LOG"
        exit 1
    fi
else
    echo "[1/5] Server already running (PID: $SERVER_PID)"
    # Copy its log
    CMDLINE=$(cat /proc/$SERVER_PID/cmdline 2>/dev/null | tr '\0' ' ' || echo "")
    echo "  Server cmd: $CMDLINE"
fi

# Initialize table if needed
echo "[2/5] Checking / initializing sbtest table..."
EXISTING=$("$CLI_BIN" cli "SELECT COUNT(*) FROM sbtest" --port "$PORT" 2>/dev/null | grep -c "row" || echo 0)
if [ "$EXISTING" -eq 0 ]; then
    echo "  Creating sbtest table with $TABLE_SIZE rows..."
    "$CLI_BIN" exec "CREATE TABLE IF NOT EXISTS sbtest (
        id INT NOT NULL, k INT NOT NULL DEFAULT 0,
        c CHAR(120) NOT NULL DEFAULT '', pad CHAR(60) NOT NULL DEFAULT '',
        PRIMARY KEY(id), INDEX idx_k(k))" --port "$PORT" 2>/dev/null

    # Insert in batches
    BATCH=1000
    for ((offset=0; offset<TABLE_SIZE; offset+=BATCH)); do
        VALUES=$(python3 -c "
import sys
for i in range($offset, min($offset+$BATCH, $TABLE_SIZE)):
    print(f'({i},{i%100},REPEAT(\"x\",120),REPEAT(\"y\",60))', end='')
    if i < min($offset+$BATCH, $TABLE_SIZE)-1:
        print(',', end='')
    else:
        print()
")
        "$CLI_BIN" exec "INSERT INTO sbtest (id,k,c,pad) VALUES $VALUES" --port "$PORT" 2>/dev/null
        echo -ne "\r  Inserted $(min $offset $TABLE_SIZE)/$TABLE_SIZE"
    done
    echo
else
    echo "  Table already has data (EXISTING=$EXISTING)"
fi

# Verify table
COUNT=$("$CLI_BIN" cli "SELECT COUNT(*) FROM sbtest" --port "$PORT" 2>/dev/null | grep -A1 "col_1" | tail -1 | tr -d ' ' || echo "?")
echo "  Table COUNT(*): $COUNT"

# Metrics collection
start_ts=$(date +%s)
end_ts=$((start_ts + HOURS * 3600))
initial_rss_kb=0
sample=0

echo "[3/5] Starting $THREADS soak workers..."

# Start metrics monitor in background
(
    while [ $(date +%s) -lt $end_ts ]; do
        elapsed=$(($(date +%s) - start_ts))
        elapsed_h=$(awk "BEGIN {printf \"%.3f\", $elapsed/3600}")
        read rss_kb fd_count <<< $(get_metrics $SERVER_PID)
        rss_mb=$(awk "BEGIN {printf \"%.1f\", $rss_kb/1024}")
        if [ $sample -eq 0 ]; then
            initial_rss_kb=$rss_kb
        fi
        rss_delta=$(awk "BEGIN {printf \"%.1f\", ($rss_kb - $initial_rss_kb)/1024}")
        running=$(pgrep -f "sqlrustgo-cli soak" 2>/dev/null | wc -l || echo 0)
        echo "$(date -u +%Y-%m-%dT%H:%M:%SZ),$elapsed_h,$rss_mb,$rss_delta,$fd_count,0,0,0,RUNNING" >> "$METRICS_FILE"
        if [ $((sample % 4)) -eq 0 ] && [ $sample -gt 0 ]; then
            remain=$((end_ts - $(date +%s)))
            echo "[s$sample $(date +%H:%M:%S)] RSS=${rss_mb}MB(d${rss_delta}) FD=${fd_count} workers=$running remain=${remain}s"
        fi
        sample=$((sample + 1))
        sleep $INTERVAL
    done
) &
MONITOR_PID=$!

# Start soak workers
declare -a WORKER_PIDS
for ((i=0; i<THREADS; i++)); do
    (
        while [ $(date +%s) -lt $end_ts ]; do
            k_val=$((i % 100))
            id_start=$(((i * 7) % 1000))
            id_end=$((id_start + 10))
            pk_id=$(((i * 17) % TABLE_SIZE))

            # SELECT COUNT
            printf "SELECT COUNT(*) FROM sbtest;\n" | "$CLI_BIN" soak --host "$HOST" --port "$PORT" 2>/dev/null | grep -q "^ROW" || true

            # SELECT by k
            printf "SELECT id, k, c FROM sbtest WHERE k=%d LIMIT 3;\n" "$k_val" | "$CLI_BIN" soak --host "$HOST" --port "$PORT" 2>/dev/null | grep -q "^ROW" || true

            # SELECT range
            printf "SELECT id, k, c, pad FROM sbtest WHERE id BETWEEN %d AND %d LIMIT 5;\n" "$id_start" "$id_end" | "$CLI_BIN" soak --host "$HOST" --port "$PORT" 2>/dev/null | grep -q "^ROW" || true

            # SELECT pk
            printf "SELECT id FROM sbtest WHERE id=%d;\n" "$pk_id" | "$CLI_BIN" soak --host "$HOST" --port "$PORT" 2>/dev/null | grep -q "^ROW" || true

            # Brief pause every 20 iterations
            if [ $((RANDOM % 40)) -eq 0 ]; then
                sleep 0.05
            fi
        done
    ) >> "$SOAK_LOG" 2>&1 &
    WORKER_PIDS+=($!)
done

echo "  Started $THREADS workers: ${WORKER_PIDS[*]}"

# Wait for all workers
for pid in "${WORKER_PIDS[@]}"; do
    wait $pid 2>/dev/null || true
done

# Stop monitor
kill $MONITOR_PID 2>/dev/null || true
wait $MONITOR_PID 2>/dev/null || true

status="COMPLETED"
if [ $(date +%s) -lt $end_ts ]; then
    status="EARLY_EXIT"
fi

# Final metrics
elapsed=$(($(date +%s) - start_ts))
read rss_kb fd_count <<< $(get_metrics $SERVER_PID)
rss_mb=$(awk "BEGIN {printf \"%.1f\", $rss_kb/1024}")

# Count results
total_rows=$(grep -c "^ROW" "$SOAK_LOG" 2>/dev/null || echo 0)
queries=$((total_rows * 4))  # 4 queries per ROW in log

echo "[4/5] Collecting results..."
echo "[5/5] Generating report..."

cat > "$REPORT_FILE" << EOF
# Soak Stability Report

**Run**: $(date -u +%Y-%m-%dT%H:%M:%SZ)
**Duration**: ${elapsed}s (target ${HOURS}h)
**Status**: $status
**Host**: ${HOST}:${PORT}
**Threads**: ${THREADS}
**Table size**: ${TABLE_SIZE}

## Throughput

| Metric | Value |
|--------|-------|
| Total ROW lines | ${total_rows} |
| Elapsed | ${elapsed}s |
| Status | $status |
| Final RSS | ${rss_mb}MB |
| Final FD | ${fd_count} |

## Verdict

**$status** — ${elapsed}s soak, rows=${total_rows}
EOF

echo "=========================================="
echo "Soak Complete"
echo "=========================================="
echo "Status: $status"
echo "Elapsed: ${elapsed}s"
echo "Total ROW lines: $total_rows"
echo "Final RSS: ${rss_mb}MB"
echo "Final FD: ${fd_count}"
echo "Report: $REPORT_FILE"
echo "Results: $RESULTS_DIR"