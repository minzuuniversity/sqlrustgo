#!/bin/bash
# run_72h_soak.sh - Real wall-clock 72h soak on Z6G4 (#3265)
#
# Steps:
# 1. Start sqlrustgo-mysql-server serve with WAL fix
# 2. Start sysbench oltp_read_write as workload
# 3. Monitor RSS/FD/CPU/WAL/disk every 60s for 72h
# 4. Write SOAK_72H_REPORT.md at end
#
# Usage:
#   bash scripts/stability/run_72h_soak.sh
#
# Stop:
#   kill $(cat /opt/sqlrustgo-soak/server.pid)
#   kill $(cat /opt/sqlrustgo-soak/sysbench.pid)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

HOURS=${HOURS:-72}
INTERVAL=${INTERVAL:-60}
RESULTS_DIR=${RESULTS_DIR:-"$HOME/sqlrustgo-soak/results_72h_$(date +%Y%m%d_%H%M%S)"}
SQLRUSTGO_BIN="$HOME/sqlrustgo-soak/bin/sqlrustgo-mysql-server"
TPCH_FIXTURE="$PROJECT_ROOT/tests/data/tpch-sf01"
LOG_FILE="$RESULTS_DIR/soak.log"
METRICS_FILE="$RESULTS_DIR/metrics.csv"
PID_FILE="$RESULTS_DIR/server.pid"
SYSBENCH_PID_FILE="$RESULTS_DIR/sysbench.pid"
REPORT_FILE="$RESULTS_DIR/SOAK_72H_REPORT.md"

mkdir -p "$RESULTS_DIR"

log() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] $*" | tee -a "$LOG_FILE"; }
alert() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] ALERT: $*" | tee -a "$LOG_FILE" >&2; }

log "=========================================="
log "Z6G4 72h Soak (#3265)"
log "Hours: $HOURS  Interval: ${INTERVAL}s"
log "Results: $RESULTS_DIR"
log "=========================================="

RSS_HARD_LIMIT_MB=${RSS_HARD_LIMIT_MB:-4096}
WAL_HARD_LIMIT_MB=${WAL_HARD_LIMIT_MB:-1024}
DISK_MIN_GB=${DISK_MIN_GB:-10}

# 1. Start server
log "[1/4] Starting server..."
if [ ! -x "$SQLRUSTGO_BIN" ]; then
    log "  ERROR: $SQLRUSTGO_BIN not found"
    exit 1
fi
nohup "$SQLRUSTGO_BIN" serve --port 13306 --data-dir "$RESULTS_DIR/data" > "$LOG_FILE.server" 2>&1 &
SERVER_PID=$!
echo $SERVER_PID > "$PID_FILE"
log "  Server PID: $SERVER_PID"

for _ in $(seq 1 30); do
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        log "  ERROR: server died on startup"
        cat "$LOG_FILE.server"
        exit 1
    fi
    sleep 1
    if nc -z 127.0.0.1 3306 2>/dev/null; then
        break
    fi
done
log "  Server ready on port 13306"

# 2. Start sysbench
log "[2/4] Starting sysbench oltp_read_write (4 threads, 72h)..."
nohup sysbench oltp_read_write \
    --db-driver=mysql \
    --mysql-host=127.0.0.1 --mysql-port=13306 \
    --mysql-user=root \
    --mysql-db=sbtest --table-size=1000 --tables=1 \
    --threads=4 --time=$(awk -v h="$HOURS" 'BEGIN{print h*3600}') \
    --report-interval=60 \
    run > "$LOG_FILE.sysbench" 2>&1 &
SYSBENCH_PID=$!
echo $SYSBENCH_PID > "$SYSBENCH_PID_FILE"
log "  sysbench PID: $SYSBENCH_PID"

# 3. Monitor
log "[3/4] Monitoring ${HOURS}h..."
echo "ts,elapsed_s,rss_mb,fd_count,cpu_pct,wal_mb,disk_avail_gb" > "$METRICS_FILE"

END_TS=$(awk -v h="$HOURS" 'BEGIN{print systime() + h*3600}')
while [ "$(date +%s)" -lt "$END_TS" ]; do
    sleep "$INTERVAL"
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        alert "Server PID $SERVER_PID died"
        cat "$LOG_FILE.server" | tail -20
        exit 1
    fi
    TS=$(date '+%Y-%m-%d %H:%M:%S')
    ELAPSED=$(($(date +%s) - $(stat -c %Y "$RESULTS_DIR")))
    RSS_MB=$(($(ps -o rss= -p $SERVER_PID 2>/dev/null | tr -d ' ') / 1024))
    FD_COUNT=$(($(lsof -p $SERVER_PID 2>/dev/null | wc -l) - 1))
    CPU_PCT=$(ps -o %cpu= -p $SERVER_PID 2>/dev/null | tr -d ' ')
    WAL_MB=$(($(stat -c %s "$RESULTS_DIR/data/sqlrustgo.wal" 2>/dev/null || echo 0) / 1024 / 1024))
    DISK_GB=$(df -BG "$RESULTS_DIR" | tail -1 | awk '{print $4}')
    echo "$TS,$ELAPSED,$RSS_MB,$FD_COUNT,$CPU_PCT,$WAL_MB,$DISK_GB" >> "$METRICS_FILE"
    log "  $TS: RSS=${RSS_MB}MB FD=${FD_COUNT} CPU=${CPU_PCT}% WAL=${WAL_MB}MB Disk=${DISK_GB}GB"
done

# 4. Cleanup + Report
log "[4/4] Generating report..."
kill -TERM $SERVER_PID 2>/dev/null || true
sleep 5
kill -KILL $SERVER_PID 2>/dev/null || true
[ -f "$SYSBENCH_PID_FILE" ] && kill -TERM $(cat $SYSBENCH_PID_FILE) 2>/dev/null || true

RSS_START=$(head -2 "$METRICS_FILE" | tail -1 | cut -d, -f3)
RSS_END=$(tail -1 "$METRICS_FILE" | cut -d, -f3)
WAL_MAX=$(awk -F, 'NR>1{print $6}' "$METRICS_FILE" | sort -n | tail -1)
FD_MAX=$(awk -F, 'NR>1{print $4}' "$METRICS_FILE" | sort -n | tail -1)
TRUNC=$(grep -c "truncating" "$LOG_FILE.server" 2>/dev/null || echo 0)

cat > "$REPORT_FILE" << EOF
# Z6G4 72h Soak Report (#3265)

- Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
- Duration: ${HOURS}h wall-clock
- Result dir: $RESULTS_DIR
- WAL fix: PR #3533

## Metrics
- RSS start/end: ${RSS_START}/${RSS_END} MB
- WAL max: ${WAL_MAX} MB (threshold 1024)
- FD max: ${FD_MAX}
- WAL truncates: $TRUNC

## Acceptance
- Crashes: 0
- WAL bounded: $([ "$WAL_MAX" -lt "$WAL_HARD_LIMIT_MB" ] && echo "YES" || echo "NO")
- Mem stable: $([ $((${RSS_END:-0} - ${RSS_START:-0})) -lt 1000 ] && echo "YES" || echo "NO")
EOF

log "Report: $REPORT_FILE"
log "=========================================="
log "Soak complete"
log "=========================================="