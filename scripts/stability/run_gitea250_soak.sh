#!/bin/bash
# run_gitea250_soak.sh — Gitea #250 72h SOAK test entry point
#
# Tests: Gitea #250 latest sqlrustgo-mysql-server + sqlrustgo-soak
# Duration: 1h by default (set HOURS=72 for 72h)
#
# Usage:
#   bash scripts/stability/run_gitea250_soak.sh            # 1h soak
#   HOURS=72 bash scripts/stability/run_gitea250_soak.sh    # 72h soak
#   QPS=10 bash scripts/stability/run_gitea250_soak.sh       # 10 QPS
#
# Stop:
#   kill $(cat $RESULTS_DIR/server.pid $RESULTS_DIR/soak.pid 2>/dev/null)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# ── Config ──────────────────────────────────────────────────────────────────
HOURS=${HOURS:-1}
DURATION_SECS=${DURATION_SECS:-$(awk "BEGIN{print int($HOURS*3600)}")}
QPS=${QPS:-5.0}
PORT=${PORT:-3396}
DATA_DIR_BASE="${DATA_DIR_BASE:-$PROJECT_ROOT/soak_gitea250_data}"
SQLRUSTGO_BIN="$PROJECT_ROOT/target/release/sqlrustgo-mysql-server"
SOAK_BIN="$PROJECT_ROOT/target/release/sqlrustgo-soak"
QUERY_FILE="$PROJECT_ROOT/scripts/soak/soak_oltp_queries.sql"

RESULTS_DIR="${DATA_DIR_BASE}/soak_$(date +%Y%m%d_%H%M%S)"
SERVER_LOG="$RESULTS_DIR/server.log"
SOAK_LOG="$RESULTS_DIR/soak.log"
METRICS_CSV="$RESULTS_DIR/metrics.csv"
REPORT="$RESULTS_DIR/SOAK_REPORT.md"

# ── Verify ──────────────────────────────────────────────────────────────────
if [[ ! -x "$SQLRUSTGO_BIN" ]]; then
    echo "ERROR: sqlrustgo-mysql-server not found at $SQLRUSTGO_BIN"
    echo "Build with: cargo build --release -p sqlrustgo-mysql-server"
    exit 1
fi

if [[ ! -x "$SOAK_BIN" ]]; then
    echo "ERROR: sqlrustgo-soak not found at $SOAK_BIN"
    echo "Build with: cargo build --release -p sqlrustgo-soak"
    exit 1
fi

mkdir -p "$RESULTS_DIR"

log() {
    echo "[$(date '+%Y-%m-%dT%H:%M:%S%z')] $*" | tee -a "$RESULTS_DIR/soak-main.log"
}

alert() {
    echo "[$(date '+%Y-%m-%dT%H:%M:%S%z')] ALERT: $*" | tee -a "$RESULTS_DIR/soak-main.log" >&2
}

log "=========================================="
log "Gitea #250 SOAK Test"
log "  Binary:     $SQLRUSTGO_BIN"
log "  Duration:   ${HOURS}h (${DURATION_SECS}s)"
log "  QPS:        $QPS"
log "  Port:       $PORT"
log "  Query file: $QUERY_FILE"
log "  Results:    $RESULTS_DIR"
log "=========================================="

# ── 1. Start server ──────────────────────────────────────────────────────────
log "[1/4] Starting sqlrustgo-mysql-server serve on port $PORT..."
DATA_DIR="$RESULTS_DIR/data"
mkdir -p "$DATA_DIR"

nohup "$SQLRUSTGO_BIN" serve \
    --port "$PORT" \
    --data-dir "$DATA_DIR" \
    > "$SERVER_LOG" 2>&1 &
SERVER_PID=$!
echo $SERVER_PID > "$RESULTS_DIR/server.pid"
log "  Server PID: $SERVER_PID"

# Wait for server to be ready
for i in $(seq 1 30); do
    if nc -z 127.0.0.1 "$PORT" 2>/dev/null; then
        log "  Server ready on port $PORT"
        break
    fi
    sleep 1
done

if ! nc -z 127.0.0.1 "$PORT" 2>/dev/null; then
    alert "Server failed to start — check $SERVER_LOG"
    exit 1
fi

# ── 2. Initialize OLTP schema ────────────────────────────────────────────────
log "[2/5] Loading OLTP schema..."
if ! mysql -h 127.0.0.1 -P "$PORT" -u root < "$PROJECT_ROOT/scripts/soak/oltp_schema.sql" 2>>"$SERVER_LOG"; then
    alert "Schema init failed — check $SERVER_LOG"
    exit 1
fi
log "  Schema loaded."

# ── 3. Seed OLTP data ────────────────────────────────────────────────────────
log "[3/5] Seeding OLTP data (small level, ~50s)..."
SEED_LOG="$RESULTS_DIR/seed.log"
LEVEL=small bash "$PROJECT_ROOT/scripts/soak/prepare_oltp_data.sh" \
    --level=small \
    --host=127.0.0.1 \
    --port="$PORT" \
    --user=root \
    > "$SEED_LOG" 2>&1
SEED_EXIT=$?
if [[ $SEED_EXIT -ne 0 ]]; then
    alert "Data seeding failed (exit=$SEED_EXIT) — check $SEED_LOG"
    exit 1
fi
log "  Data seeded."

# Verify tables
CUST_COUNT=$(mysql -h 127.0.0.1 -P "$PORT" -u root -e "SELECT COUNT(*) FROM customers" 2>/dev/null | tail -1)
ORD_COUNT=$(mysql -h 127.0.0.1 -P "$PORT" -u root -e "SELECT COUNT(*) FROM orders" 2>/dev/null | tail -1)
ITEM_COUNT=$(mysql -h 127.0.0.1 -P "$PORT" -u root -e "SELECT COUNT(*) FROM order_items" 2>/dev/null | tail -1)
if [[ -z "$CUST_COUNT" || -z "$ORD_COUNT" || -z "$ITEM_COUNT" ]]; then
    alert "Data verification failed — tables not properly seeded"
    exit 1
fi
log "  Data verified: $CUST_COUNT customers, $ORD_COUNT orders, $ITEM_COUNT items."

# ── 4. Start metrics collector ────────────────────────────────────────────────
log "[4/5] Starting metrics collector (pid=$SERVER_PID)..."
echo "ts_epoch,ts,elapsed_s,rss_mb,fd_count,cpu_pct" > "$METRICS_CSV"

METRICS_RUNNING=true
collect_metrics() {
    local t0=$(date +%s)
    while [[ "$METRICS_RUNNING" == "true" ]]; do
        local ts_epoch
        ts_epoch=$(date +%s)
        local ts
        ts=$(date '+%Y-%m-%dT%H:%M:%S')
        local elapsed=$(( ts_epoch - t0 ))

        # RSS
        local rss_mb="N/A"
        if [[ -d "/proc/$SERVER_PID" ]]; then
            rss_mb=$(awk '/VmRSS/{print int($2/1024)}' /proc/$SERVER_PID/status 2>/dev/null || echo "N/A")
        fi

        # FD count
        local fd_count="N/A"
        if [[ -d "/proc/$SERVER_PID/fd" ]]; then
            fd_count=$(ls /proc/$SERVER_PID/fd 2>/dev/null | wc -l || echo "N/A")
        fi

        # CPU (approximate via top)
        local cpu_pct="N/A"
        if [[ -d "/proc/$SERVER_PID" ]]; then
            cpu_pct=$(top -bn1 -p "$SERVER_PID" 2>/dev/null | awk -v p="$SERVER_PID" '$1==p {print $9}' | head -1 || echo "N/A")
        fi

        echo "$ts_epoch,$ts,$elapsed,$rss_mb,$fd_count,$cpu_pct" >> "$METRICS_CSV"

        # Alert on high RSS
        if [[ "$rss_mb" != "N/A" ]] && [[ "$rss_mb" -gt 4096 ]]; then
            alert "RSS ${rss_mb}MB exceeds 4GB threshold!"
        fi

        sleep 10
    done
}

collect_metrics &
METRICS_PID=$!
echo $METRICS_PID > "$RESULTS_DIR/metrics.pid"

# ── 3. Start soak ────────────────────────────────────────────────────────────
log "[3/4] Starting sqlrustgo-soak soak (${DURATION_SECS}s @ ${QPS} QPS)..."

"$SOAK_BIN" soak \
    --host 127.0.0.1 \
    --port "$PORT" \
    --user root \
    --password "" \
    --duration "$DURATION_SECS" \
    --rate "$QPS" \
    --query-file "$QUERY_FILE" \
    --report-interval 60 \
    > "$SOAK_LOG" 2>&1 &
SOAK_PID=$!
echo $SOAK_PID > "$RESULTS_DIR/soak.pid"
log "  Soak PID: $SOAK_PID"

# ── 5. Start soak ────────────────────────────────────────────────────────────
log "[5/5] Starting sqlrustgo-soak soak (${DURATION_SECS}s @ ${QPS} QPS)..."

# ── Wait for soak completion (follows step 5) ───────────────────────────────
log "Waiting for soak to complete (${HOURS}h wall-clock)..."

wait $SOAK_PID
SOAK_EXIT=$?
log "  Soak exited with code: $SOAK_EXIT"

# ── Stop metrics + server ─────────────────────────────────────────────────────
METRICS_RUNNING=false
sleep 2
kill $METRICS_PID 2>/dev/null || true
kill $SERVER_PID 2>/dev/null || true
sleep 2
kill -9 $SERVER_PID 2>/dev/null || true

# ── Generate report ───────────────────────────────────────────────────────────
log "=========================================="
log "Generating report..."
log "=========================================="

# Parse soak log for results
TOTAL_Q=""
ERRORS=""
AVG_LAT=""
P99_LAT=""

if [[ -f "$SOAK_LOG" ]]; then
    # Try to extract metrics from human-readable output
    TOTAL_Q=$(grep -i "queries" "$SOAK_LOG" | grep -v "^#" | awk '{print $NF}' | head -1 || echo "N/A")
    ERRORS=$(grep -i "error" "$SOAK_LOG" | grep -v "^#" | awk '{print $NF}' | head -1 || echo "0")
fi

# Metrics summary
RSS_START=$(sed -n '2p' "$METRICS_CSV" 2>/dev/null | cut -d, -f4 || echo "N/A")
RSS_END=$(tail -1 "$METRICS_CSV" 2>/dev/null | cut -d, -f4 || echo "N/A")
RSS_PEAK=$(awk -F, 'NR>1 && $4!="N/A"{print $4}' "$METRICS_CSV" 2>/dev/null | sort -n | tail -1 || echo "N/A")
FD_PEAK=$(awk -F, 'NR>1 && $5!="N/A"{print $5}' "$METRICS_CSV" 2>/dev/null | sort -n | tail -1 || echo "N/A")

cat > "$REPORT" << EOF
# Gitea #250 SOAK Test Report
Generated: $(date '+%Y-%m-%d %H:%M:%S %Z')

## Test Configuration
| Item | Value |
|------|-------|
| Binary | $SQLRUSTGO_BIN |
| Git Commit | $(git -C "$PROJECT_ROOT" rev-parse --short HEAD 2>/dev/null || echo "N/A") |
| Gitea Commit | $(git -C "$PROJECT_ROOT" log -1 --format="%H %s" 2>/dev/null || echo "N/A") |
| Duration | ${HOURS}h (${DURATION_SECS}s wall-clock) |
| Target QPS | $QPS |
| Port | $PORT |
| Results Dir | $RESULTS_DIR |

## Results Summary
| Metric | Value |
|--------|-------|
| Soak Exit Code | $SOAK_EXIT |
| Total Queries | $TOTAL_Q |
| Errors | $ERRORS |
| RSS Start (MB) | $RSS_START |
| RSS End (MB) | $RSS_END |
| RSS Peak (MB) | $RSS_PEAK |
| FD Peak | $FD_PEAK |

## Resource Timeline
$(cat "$METRICS_CSV")

## Soak Log
$(cat "$SOAK_LOG")

EOF

log "Report: $REPORT"
log "Metrics: $METRICS_CSV"
log "Soak log: $SOAK_LOG"
log "Server log: $SERVER_LOG"

if [[ "$SOAK_EXIT" -ne 0 ]]; then
    alert "Soak exited with non-zero code: $SOAK_EXIT"
    exit $SOAK_EXIT
fi

log "SOAK COMPLETE — exit=$SOAK_EXIT"
exit 0
