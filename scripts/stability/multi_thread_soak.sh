#!/bin/bash
set -euo pipefail

HOST=${HOST:-127.0.0.1}
PORT=${PORT:-13399}
USER=${USER:-root}
PASSWORD=${PASSWORD:-}
THREADS=${THREADS:-4}
DURATION=${DURATION:-60}
TABLE_SIZE=${TABLE_SIZE:-1000}
RESULTS_DIR=${RESULTS_DIR:-"/tmp/soak_results_$(date +%Y%m%d_%H%M%S)"}
SQLRUSTGO_BIN=${SQLRUSTGO_BIN:-./target/release/sqlrustgo}

mkdir -p "$RESULTS_DIR"

echo "=========================================="
echo "Multi-thread Soak Test (sqlrustgo-cli based)"
echo "Host: $HOST:$PORT  Threads: $THREADS  Duration: ${DURATION}s"
echo "=========================================="

cleanup() {
    for pid in "${THREAD_PIDS[@]:-}"; do kill "$pid" 2>/dev/null || true; done
    kill $SERVER_PID 2>/dev/null || true
}
trap cleanup EXIT

if ! lsof -i ":$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "[1/4] Starting server..."
    rm -rf "$RESULTS_DIR/data" && mkdir -p "$RESULTS_DIR/data"
    nohup "$SQLRUSTGO_BIN" serve --port "$PORT" --data-dir "$RESULTS_DIR/data" > "$RESULTS_DIR/server.log" 2>&1 &
    SERVER_PID=$!
    sleep 3
    kill -0 $SERVER_PID || { cat "$RESULTS_DIR/server.log"; exit 1; }
    echo "  Server PID: $SERVER_PID"
else
    echo "[1/4] Server already running on port $PORT"
    SERVER_PID=""
fi

echo "[2/4] Initializing database..."
{
    echo "CREATE DATABASE IF NOT EXISTS sbtest; USE sbtest;"
    echo "DROP TABLE IF EXISTS sbtest1;"
    echo "CREATE TABLE sbtest1 (id INT PRIMARY KEY, k INT DEFAULT 0, c CHAR(120) DEFAULT '', pad CHAR(60) DEFAULT '');"
} | "$SQLRUSTGO_BIN" soak --host "$HOST" --port "$PORT" -u "$USER" -w "$PASSWORD" > /dev/null 2>&1

echo "[3/4] Inserting $TABLE_SIZE rows..."
for i in $(seq 1 $TABLE_SIZE); do
    echo "INSERT INTO sbtest1 (id, k, c, pad) VALUES ($i, $((i % 100)), 'p-$i', 'pad-$i');"
done | "$SQLRUSTGO_BIN" soak --host "$HOST" --port "$PORT" -u "$USER" -w "$PASSWORD" > /dev/null 2>&1
echo "  Done"

declare -a THREAD_PIDS
declare -a THREAD_LOGS
START_TIME=$(date +%s)

echo "[4/4] Starting $THREADS worker threads..."
for t in $(seq 1 $THREADS); do
    LOG="$RESULTS_DIR/thread_${t}.log"
    THREAD_LOGS[$t]="$LOG"
    (
        c=0; e=0
        while [ $(($(date +%s) - $START_TIME)) -lt $DURATION ]; do
            id=$((RANDOM % TABLE_SIZE + 1))
            result=$("$SQLRUSTGO_BIN" soak --host "$HOST" --port "$PORT" -u "$USER" -w "$PASSWORD" 2>&1 <<< "SELECT k FROM sbtest1 WHERE id=$id;" | head -1)
            if echo "$result" | grep -q "^ERR"; then
                e=$((e + 1))
            else
                c=$((c + 1))
            fi
        done
        echo "OK:$c ERR:$e" >> "$LOG"
    ) &
    THREAD_PIDS[$t]=$!
done

echo "Monitoring..."
METRICS="$RESULTS_DIR/metrics.csv"
echo "ts,elapsed_s,alive,queries,errors" > "$METRICS"

while [ $(($(date +%s) - START_TIME)) -lt $DURATION ]; do
    sleep 5
    ELAPSED=$(($(date +%s) - START_TIME))
    alive=0
    for pid in "${THREAD_PIDS[@]}"; do kill -0 $pid 2>/dev/null && alive=$((alive + 1)); done
    queries=0; errors=0
    for log in "${THREAD_LOGS[@]}"; do
        [ -f "$log" ] && { last=$(tail -1 "$log"); q=$(echo "$last" | sed 's/.*OK:\([0-9]*\).*/\1/'); e=$(echo "$last" | sed 's/.*ERR:\([0-9]*\).*/\1/'); queries=$((queries + ${q:-0})); errors=$((errors + ${e:-0})); }
    done
    echo "$(date +%s),$ELAPSED,$alive,$queries,$errors" >> "$METRICS"
    echo "  [${ELAPSED}s] alive=$alive queries=$queries errors=$errors"
done

for pid in "${THREAD_PIDS[@]}"; do wait $pid 2>/dev/null || true; done

total_queries=0; total_errors=0
for log in "${THREAD_LOGS[@]}"; do
    [ -f "$log" ] && { last=$(tail -1 "$log"); q=$(echo "$last" | sed 's/.*OK:\([0-9]*\).*/\1/'); e=$(echo "$last" | sed 's/.*ERR:\([0-9]*\).*/\1/'); total_queries=$((total_queries + ${q:-0})); total_errors=$((total_errors + ${e:-0})); }
done

cat > "$RESULTS_DIR/REPORT.md" << EOF
# Multi-thread Soak Test Report

## Config
- Host: $HOST:$PORT  Threads: $THREADS  Duration: ${DURATION}s  Table: $TABLE_SIZE

## Results
- Total queries: $total_queries
- Total errors: $total_errors
- QPS: $(awk "BEGIN {printf \"%.2f\", $total_queries / $DURATION}")

## Stability
$(if [ $total_errors -eq 0 ]; then echo "✅ PASS"; else echo "⚠️ WARN: $total_errors errors"; fi)
EOF

cat "$RESULTS_DIR/REPORT.md"
echo "✅ Results: $RESULTS_DIR"
