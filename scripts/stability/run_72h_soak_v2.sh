#!/bin/bash
# run_72h_soak_v2.sh - Real wall-clock 72h soak on Z6G4 (#3265)
#
# Uses the in-process `soak` subcommand (no MySQL protocol needed).
# Also runs the `serve` subcommand in parallel for server stability.
#
# Start:  nohup bash run_72h_soak_v2.sh > soak.log 2>&1 & disown
# Stop:   kill $(cat results_*/server.pid results_*/soak.pid 2>/dev/null)

set -uo pipefail

HOURS=${HOURS:-72}
QPS=${QPS:-1.0}
INTERVAL=${INTERVAL:-60}
RESULTS_DIR=${RESULTS_DIR:-"$HOME/sqlrustgo-soak/soak72h_$(date +%Y%m%d_%H%M%S)"}
SQLRUSTGO_BIN="$HOME/sqlrustgo-soak/bin/sqlrustgo-mysql-server"
SERVER_LOG="$RESULTS_DIR/server.log"
SOAK_LOG="$RESULTS_DIR/soak.log"
SERVER_METRICS="$RESULTS_DIR/server_metrics.csv"
SOAK_METRICS="$RESULTS_DIR/soak_metrics.csv"
REPORT="$RESULTS_DIR/SOAK_72H_REPORT.md"

mkdir -p "$RESULTS_DIR"

log() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] $*" | tee -a "$RESULTS_DIR/soak-main.log"; }
alert() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] ALERT: $*" | tee -a "$RESULTS_DIR/soak-main.log" >&2; }

log "=========================================="
log "Z6G4 72h Soak (#3265) v2"
log "Hours: $HOURS  QPS: $QPS  Interval: ${INTERVAL}s"
log "Results: $RESULTS_DIR"
log "=========================================="

# 1. Start MySQL server (for server-stability monitoring)
log "[1/3] Starting sqlrustgo-mysql-server serve on port 13306..."
nohup "$SQLRUSTGO_BIN" serve --port 13306 --data-dir "$RESULTS_DIR/data" > "$SERVER_LOG" 2>&1 &
SERVER_PID=$!
echo $SERVER_PID > "$RESULTS_DIR/server.pid"
log "  Server PID: $SERVER_PID"

# Wait for ready
for _ in $(seq 1 30); do
    if nc -z 127.0.0.1 13306 2>/dev/null; then
        break
    fi
    sleep 1
done
log "  Server ready on port 13306"

# 2. Start in-process soak subcommand (in-process, no protocol needed)
log "[2/3] Starting soak subcommand (in-process, --qps $QPS)..."
nohup "$SQLRUSTGO_BIN" soak --duration "$HOURS" --qps "$QPS" \
    --output "$RESULTS_DIR/soak.jsonl" \
    --sample-interval-s "$INTERVAL" > "$SOAK_LOG" 2>&1 &
SOAK_PID=$!
echo $SOAK_PID > "$RESULTS_DIR/soak.pid"
log "  Soak PID: $SOAK_PID"

# 3. Monitor server resources in parallel
log "[3/3] Monitoring server..."
echo "ts,elapsed_s,rss_mb,fd_count,cpu_pct,wal_mb,disk_avail_gb" > "$SERVER_METRICS"

RSS_HARD_LIMIT_MB=${RSS_HARD_LIMIT_MB:-4096}
WAL_HARD_LIMIT_MB=${WAL_HARD_LIMIT_MB:-1024}
DISK_MIN_GB=${DISK_MIN_GB:-10}

END_TS=$(awk -v h="$HOURS" 'BEGIN{print systime() + h*3600}')
while [ "$(date +%s)" -lt "$END_TS" ]; do
    sleep "$INTERVAL"
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        alert "Server PID $SERVER_PID died"
        cat "$SERVER_LOG" | tail -20
        exit 1
    fi
    if ! kill -0 "$SOAK_PID" 2>/dev/null; then
        alert "Soak PID $SOAK_PID died (early!)"
        cat "$SOAK_LOG" | tail -20
        exit 1
    fi
    TS=$(date '+%Y-%m-%d %H:%M:%S')
    ELAPSED=$(($(date +%s) - $(stat -c %Y "$RESULTS_DIR")))
    RSS_MB=$(($(ps -o rss= -p $SERVER_PID 2>/dev/null | tr -d ' ') / 1024))
    FD_COUNT=$(($(lsof -p $SERVER_PID 2>/dev/null | wc -l) - 1))
    CPU_PCT=$(ps -o %cpu= -p $SERVER_PID 2>/dev/null | tr -d ' ')
    WAL_MB=$(($(stat -c %s "$RESULTS_DIR/data/sqlrustgo.wal" 2>/dev/null || echo 0) / 1024 / 1024))
    DISK_GB=$(df -BG "$RESULTS_DIR" | tail -1 | awk '{print $4}')
    echo "$TS,$ELAPSED,$RSS_MB,$FD_COUNT,$CPU_PCT,$WAL_MB,$DISK_GB" >> "$SERVER_METRICS"
    log "  $TS: server RSS=${RSS_MB}MB FD=${FD_COUNT} CPU=${CPU_PCT}% WAL=${WAL_MB}MB Disk=${DISK_GB}GB"
done

# Cleanup + Report
log "=========================================="
log "Soak complete - generating report"
log "=========================================="
kill -TERM $SERVER_PID 2>/dev/null || true
kill -TERM $SOAK_PID 2>/dev/null || true
sleep 5
kill -KILL $SERVER_PID 2>/dev/null || true
kill -KILL $SOAK_PID 2>/dev/null || true

RSS_START=$(head -2 "$SERVER_METRICS" | tail -1 | cut -d, -f3)
RSS_END=$(tail -1 "$SERVER_METRICS" | cut -d, -f3)
WAL_MAX=$(awk -F, 'NR>1{print $6}' "$SERVER_METRICS" | sort -n | tail -1)

cat > "$REPORT" << EOF
# Z6G4 72h Soak Report (#3265) v2

- Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
- Duration: ${HOURS}h wall-clock
- QPS: $QPS
- Result dir: $RESULTS_DIR
- Mode: in-process soak subcommand + parallel MySQL server
- WAL fix: PR #3533

## Server metrics
- RSS start/end: ${RSS_START}/${RSS_END} MB
- WAL max: ${WAL_MAX} MB (threshold 1024)

## Acceptance
- Server alive: $([ -n "$SERVER_PID" ] && echo "tracked" || echo "n/a")
- Soak completed: $([ -f "$RESULTS_DIR/soak.jsonl" ] && echo "YES" || echo "NO")
- WAL bounded: $([ "$WAL_MAX" -lt "$WAL_HARD_LIMIT_MB" ] && echo "YES" || echo "NO")
EOF

cat "$SERVER_METRICS" >> "$REPORT"
[ -f "$RESULTS_DIR/soak.jsonl" ] && cat "$RESULTS_DIR/soak.jsonl" >> "$REPORT"
log "Report: $REPORT"