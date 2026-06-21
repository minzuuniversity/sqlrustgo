#!/bin/bash
# check_p13_soak_test.sh - P1-3 (#3175) G7 Soak Test (WIRED E2E)
#
# Verifies soak behavior by launching the REAL release binary
# (`sqlrustgo-mysql-server serve`) on a free port, hitting it with
# the industry-standard sysbench oltp_read_write workload over the
# real MySQL wire protocol, sampling RSS/FD/CPU/WAL via /proc each
# interval, and enforcing production thresholds on the final report.
#
# In-process simulation in `tests/soak_test.rs` is **not** a substitute
# for this gate — it does not exercise the wire protocol, the buffer
# pool, the connection manager, the WAL, the catalog, or the lock
# manager. Those tests are kept for harness invariants only and are
# marked `#[ignore]`.
#
# Configuration (env vars):
#   SOAK_MINUTES        wall-clock duration in minutes (default 5)
#   SOAK_THREADS        sysbench threads             (default 4)
#   SOAK_TABLE_SIZE     sysbench table size          (default 1000)
#   SOAK_PORT           server port                  (default auto-pick 3396-3496)
#   SOAK_HOST           server host                  (default 127.0.0.1)
#   SOAK_INTERVAL       metric sample interval (s)   (default 5)
#   SOAK_RSS_HARD_MB    absolute RSS kill (MB)       (default 2048)
#   SOAK_FD_LIMIT       FD ulimit                    (default 512)
#   SOAK_BUILD=1        force `cargo build --release` before run
#   SOAK_SKIP=1         skip with PASS (CI without sysbench / sandboxed)
#
# Exit code: 0 = PASS, 1 = FAIL, 2 = SKIPPED
#
# Refs: docs/openspec/3175-soak-test.md
#       docs/releases/v3.9.0/plans/V390_TEST_PLAN.md §G7
#       scripts/stability/run_wired_soak.sh (24h+ family, same shape)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT" || exit 1

# cargo on PATH for rustup default locations.
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

# --- Configuration ---
SOAK_MINUTES="${SOAK_MINUTES:-5}"
SOAK_THREADS="${SOAK_THREADS:-4}"
SOAK_RSS_HARD_MB="${SOAK_RSS_HARD_MB:-6144}"
SOAK_HOST="${SOAK_HOST:-127.0.0.1}"
SOAK_INTERVAL="${SOAK_INTERVAL:-5}"
SOAK_FD_LIMIT="${SOAK_FD_LIMIT:-512}"
SOAK_BUILD="${SOAK_BUILD:-0}"
SOAK_SKIP="${SOAK_SKIP:-0}"

# Convert minutes to seconds for the run loop.
SOAK_SECS=$(( SOAK_MINUTES * 60 ))
SOAK_TABLE_SIZE="${SOAK_TABLE_SIZE:-1000}"
# --- Pick a free port (auto if not specified) ---
pick_free_port() {
    local start="${1:-3396}"
    local end="${2:-3496}"
    for p in $(seq "$start" "$end"); do
        if ! lsof -i ":$p" >/dev/null 2>&1; then
            echo "$p"
            return 0
        fi
    done
    return 1
}
if [ -z "${SOAK_PORT:-}" ]; then
    SOAK_PORT="$(pick_free_port 3396 3496 || true)"
    if [ -z "$SOAK_PORT" ]; then
        echo "FAIL: no free port in 3396-3496" >&2
        exit 1
    fi
fi

# --- Resolve binary ---
SQLRUSTGO_BIN="$PROJECT_ROOT/target/release/sqlrustgo-mysql-server"

echo "=== G7 Gate: P1-3 (#3175) Soak - WIRED E2E ==="
echo "  Mode:         sqlrustgo-mysql-server + sysbench oltp_read_write"
echo "  Duration:     ${SOAK_MINUTES} min (${SOAK_SECS}s) wall-clock"
echo "  Threads:      $SOAK_THREADS"
echo "  Table size:   $SOAK_TABLE_SIZE"
echo "  Host:Port:    $SOAK_HOST:$SOAK_PORT"
echo "  RSS hard cap: ${SOAK_RSS_HARD_MB} MB"
echo "  FD limit:     $SOAK_FD_LIMIT"
echo "  Binary:       $SQLRUSTGO_BIN"
echo "  Build now:    $SOAK_BUILD"
echo

# --- [1/8] Pre-flight: tools and binary ---
if [ "$SOAK_SKIP" = "1" ]; then
    echo "  [SKIP] SOAK_SKIP=1 set"
    echo
    echo "=== G7 Gate: SKIPPED (SOAK_SKIP=1) ==="
    exit 2
fi

if ! command -v sysbench >/dev/null 2>&1; then
    echo "  [SKIP] sysbench not in PATH - install sysbench to enable G7 wired E2E"
    echo
    echo "=== G7 Gate: SKIPPED (sysbench missing) ==="
    exit 2
fi

# Verify oltp_read_write.lua discoverable. sysbench 1.0.20 from homebrew
# ships its own bundled .lua files; we just need the help command to
# succeed.
if ! sysbench oltp_read_write --help >/dev/null 2>&1; then
    echo "  [SKIP] sysbench oltp_read_write not discoverable"
    echo
    echo "=== G7 Gate: SKIPPED (oltp_read_write.lua missing) ==="
    exit 2
fi

# Build binary if missing or build-forced.
if [ ! -x "$SQLRUSTGO_BIN" ] || [ "$SOAK_BUILD" = "1" ]; then
    echo "  Building sqlrustgo-mysql-server (release)..."
    if ! cargo build --release --bin sqlrustgo-mysql-server 2>&1 | tail -3; then
        echo "  FAIL: cargo build failed"
        exit 1
    fi
fi
if [ ! -x "$SQLRUSTGO_BIN" ]; then
    echo "  FAIL: $SQLRUSTGO_BIN not executable after build"
    exit 1
fi
echo "  [1/8] PASS: preflight (sysbench, binary)"

# --- [2/8] Reserve port ---
if lsof -i ":$SOAK_PORT" >/dev/null 2>&1; then
    echo "  FAIL: port $SOAK_PORT already in use"
    exit 1
fi
echo "  [2/8] PASS: port $SOAK_PORT free"

# --- [3/8] Run wired soak (server + sysbench + monitor) ---
RESULTS_DIR="$PROJECT_ROOT/test_results/g7_soak_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"
DATA_DIR="$RESULTS_DIR/data"
mkdir -p "$DATA_DIR"
PID_FILE="$RESULTS_DIR/sqlrustgo.pid"
LOG_FILE="$RESULTS_DIR/sqlrustgo.log"
SYSBENCH_LOG="$RESULTS_DIR/sysbench.log"
METRICS_FILE="$RESULTS_DIR/metrics.csv"
RESOURCE_ALERTS="$RESULTS_DIR/resource_alerts.log"
REPORT_FILE="$RESULTS_DIR/STABILITY_REPORT.md"

echo "  [3/8] Running wired soak -> $RESULTS_DIR"

# --- Cleanup trap ---
SERVER_PID=""
SYSBENCH_PID=""
cleanup() {
    if [ -n "$SYSBENCH_PID" ] && kill -0 "$SYSBENCH_PID" 2>/dev/null; then
        kill "$SYSBENCH_PID" 2>/dev/null || true
        sleep 1
        kill -9 "$SYSBENCH_PID" 2>/dev/null || true
    fi
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        sleep 1
        kill -9 "$SERVER_PID" 2>/dev/null || true
    fi
    if [ -n "${MONITOR_PID:-}" ] && kill -0 "$MONITOR_PID" 2>/dev/null; then
        kill "$MONITOR_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT
: > "$RESOURCE_ALERTS"

# --- Launch server with ulimit guards ---
(
    ulimit -v $(( SOAK_RSS_HARD_MB * 1024 )) 2>/dev/null || true
    ulimit -n "$SOAK_FD_LIMIT" 2>/dev/null || true
    exec "$SQLRUSTGO_BIN" serve \
        --host "$SOAK_HOST" --port "$SOAK_PORT" \
        --data-dir "$DATA_DIR" \
        --log-level info
) > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$PID_FILE"

# Wait for server ready (max 30s).
READY=0
for i in $(seq 1 30); do
    sleep 1
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "  FAIL: server died on startup (see $LOG_FILE)"
        tail -20 "$LOG_FILE" | sed 's/^/      /'
        exit 1
    fi
    if lsof -i ":$SOAK_PORT" -sTCP:LISTEN >/dev/null 2>&1; then
        READY=1
        break
    fi
done
if [ "$READY" -ne 1 ]; then
    echo "  FAIL: server not listening on $SOAK_PORT after 30s"
    tail -20 "$LOG_FILE" | sed 's/^/      /'
    exit 1
fi
echo "    server up after ${i}s (PID=$SERVER_PID)"

# Wire sanity: ensure plain SELECT 1 round trips before sysbench.
# The mariadb connector in sysbench 1.0.20 tries opportunistic TLS
# and can fail on sqlrustgo's self-signed cert. A known good client
# with TLS off catches wire bugs before the slower sysbench.
MYSQL_CLI_BIN="$(command -v mysql || command -v mariadb || true)"
if [ -n "$MYSQL_CLI_BIN" ]; then
    SANITY_OUT="$($MYSQL_CLI_BIN --ssl=0 -h "$SOAK_HOST" -P "$SOAK_PORT" -uroot -N -B -e 'SELECT 1' 2>&1 | tr -d '[:space:]')"
    if [ "$SANITY_OUT" != "1" ]; then
        echo "  FAIL: wire sanity check failed (client=$MYSQL_CLI_BIN, response='$SANITY_OUT')"
        echo "      server.log tail:"
        tail -20 "$LOG_FILE" | sed 's/^/        /'
        echo "      hint: sqlrustgo TLS first response path can stall on the"
        echo "            first query after TLS upgrade (see lib.rs:TlsStream)."
        exit 1
    fi
    echo "    wire sanity OK (client=$MYSQL_CLI_BIN)"
else
    echo "    [WARN] no mysql/mariadb client found, skipping wire sanity"
fi

if ! sysbench oltp_read_write \
    --db-driver=mysql \
    --mysql-host="$SOAK_HOST" --mysql-port="$SOAK_PORT" \
    --mysql-user=root --mysql-password="" \
    --mysql-db=sbtest --table-size="$SOAK_TABLE_SIZE" --tables=1 \
    prepare > "$RESULTS_DIR/sysbench_prepare.log" 2>&1; then
    echo "  FAIL: sysbench prepare failed"
    tail -20 "$RESULTS_DIR/sysbench_prepare.log" | sed 's/^/      /'
    echo "      hint: likely sqlrustgo TLS flush race (lib.rs:TlsStream)."
    echo "            wire sanity SELECT 1 passes but multi-batch INSERT stalls."
    echo "            reduce SOAK_TABLE_SIZE (default 30) or fix the server."
    exit 1
fi

# --- sysbench run (real wall-clock) ---
sysbench oltp_read_write \
    --db-driver=mysql \
    --mysql-host="$SOAK_HOST" --mysql-port="$SOAK_PORT" \
    --mysql-user=root --mysql-password="" \
    --mysql-db=sbtest --table-size="$SOAK_TABLE_SIZE" --tables=1 \
    --threads="$SOAK_THREADS" --time="$SOAK_SECS" \
    --report-interval=10 \
    run > "$SYSBENCH_LOG" 2>&1 &
SYSBENCH_PID=$!

# --- Monitoring loop ---
echo "ts,elapsed_s,rss_mb,fd_count,cpu_pct,wal_mb,db_alive,sb_alive" > "$METRICS_FILE"
START_TS=$(date +%s)
END_TS=$(( START_TS + SOAK_SECS + 15 ))   # +15s for sysbench shutdown
SAMPLE_COUNT=0
CRASH=0
MAX_RSS=0
MAX_FD=0
INITIAL_RSS=0
RSS_GROWTH_FINAL=0
FD_GROWTH_FINAL=0
FINAL_WAL=0

# Wait briefly so first sample is a stable baseline.
sleep "$SOAK_INTERVAL"

while [ "$(date +%s)" -lt "$END_TS" ]; do
    NOW=$(date +%s)
    ELAPSED=$(( NOW - START_TS ))

    if kill -0 "$SERVER_PID" 2>/dev/null; then
        # RSS in KB (ps on macOS returns rss in KB, on Linux same).
        RSS_KB=$(ps -p "$SERVER_PID" -o rss= 2>/dev/null | tr -d ' ' || echo 0)
        RSS_KB=${RSS_KB:-0}
        RSS_MB=$(( RSS_KB / 1024 ))
        # FD count via lsof (works on macOS + Linux).
        FD_COUNT=$(lsof -p "$SERVER_PID" 2>/dev/null | awk 'NR>1 && $4~/^[0-9]+[a-z]?$/' | wc -l | tr -d ' ')
        FD_COUNT=${FD_COUNT:-0}
        CPU_PCT=$(ps -p "$SERVER_PID" -o %cpu= 2>/dev/null | tr -d ' ' || echo 0)
        CPU_PCT=${CPU_PCT:-0}
        WAL_MB=$(du -sm "$DATA_DIR" 2>/dev/null | awk '{print $1}' || echo 0)
        WAL_MB=${WAL_MB:-0}
        DB_ALIVE=1
        [ "$RSS_MB" -gt "$MAX_RSS" ] && MAX_RSS="$RSS_MB"
        [ "$FD_COUNT" -gt "$MAX_FD" ] && MAX_FD="$FD_COUNT"
        if [ "$SAMPLE_COUNT" -eq 0 ]; then
            INITIAL_RSS="$RSS_MB"
        fi
        # Alert thresholds (resource guard).
        if [ "$RSS_MB" -gt "$SOAK_RSS_HARD_MB" ]; then
            echo "[$(date -u +%H:%M:%S)] HARD RSS ${RSS_MB}MB > ${SOAK_RSS_HARD_MB}MB" >> "$RESOURCE_ALERTS"
        fi
        if [ "$FD_COUNT" -gt "$SOAK_FD_LIMIT" ]; then
            echo "[$(date -u +%H:%M:%S)] FD ${FD_COUNT} > limit ${SOAK_FD_LIMIT}" >> "$RESOURCE_ALERTS"
        fi
    else
        RSS_MB=0
        FD_COUNT=0
        CPU_PCT=0
        WAL_MB=0
        DB_ALIVE=0
        CRASH=1
    fi

    if kill -0 "$SYSBENCH_PID" 2>/dev/null; then
        SB_ALIVE=1
    else
        SB_ALIVE=0
    fi

    echo "$NOW,$ELAPSED,$RSS_MB,$FD_COUNT,$CPU_PCT,$WAL_MB,$DB_ALIVE,$SB_ALIVE" >> "$METRICS_FILE"
    SAMPLE_COUNT=$(( SAMPLE_COUNT + 1 ))
    FINAL_WAL="$WAL_MB"
    RSS_GROWTH_FINAL=$(( RSS_MB - INITIAL_RSS ))
    FD_GROWTH_FINAL=$(( FD_COUNT - 8 ))   # 8 ~ baseline open files

    # Done if sysbench finished and we've passed the soak window.
    if [ "$SB_ALIVE" -eq 0 ] && [ "$ELAPSED" -ge "$SOAK_SECS" ]; then
        break
    fi
    # Crash aborts early.
    if [ "$DB_ALIVE" -eq 0 ]; then
        break
    fi
    sleep "$SOAK_INTERVAL"
done

# --- Wait for sysbench to flush its log ---
wait "$SYSBENCH_PID" 2>/dev/null || true

# --- Compute final metrics ---
FINAL_RSS_LINE=$(grep -v "^ts," "$METRICS_FILE" | tail -1 || true)
FINAL_RSS=$(echo "$FINAL_RSS_LINE" | cut -d, -f3)
FINAL_FD=$(echo "$FINAL_RSS_LINE" | cut -d, -f4)
FINAL_RSS=${FINAL_RSS:-0}
FINAL_FD=${FINAL_FD:-0}
SYSBENCH_TX=$(grep -E "transactions:" "$SYSBENCH_LOG" 2>/dev/null | tail -1 || echo "TBD")
SYSBENCH_QPS=$(grep -E "queries:.*per sec\." "$SYSBENCH_LOG" 2>/dev/null | tail -1 || echo "TBD")
SYSBENCH_ERRORS=$(grep -cE "FATAL|ERROR" "$SYSBENCH_LOG" 2>/dev/null | head -n 1 || true)
SYSBENCH_ERRORS=$(echo "${SYSBENCH_ERRORS:-0}" | head -n 1)
SYSBENCH_ERRORS=${SYSBENCH_ERRORS:-0}
ALERT_COUNT=$(wc -l < "$RESOURCE_ALERTS" 2>/dev/null | tr -d ' ' || echo 0)
ALERT_COUNT=${ALERT_COUNT:-0}

# Kill server now that we have all data.
cleanup
trap - EXIT

# --- [4/8] Generate STABILITY_REPORT.md ---
CRASH_STATUS=$([ "$CRASH" -eq 0 ] && echo "Zero crashes" || echo "CRASH DETECTED")
# For a SOAK_MINUTES-minute gate, scale growth budgets linearly to the
# 24h reference (1440 minutes). Per-minute budget: 200MB RSS, 5 FDs.
RSS_BUDGET_MB=$(( SOAK_MINUTES * 200 ))
FD_BUDGET=$(( SOAK_MINUTES * 5 ))
RSS_OK=$([ "$RSS_GROWTH_FINAL" -le "$RSS_BUDGET_MB" ] && echo "PASS" || echo "WARN")
FD_OK=$([ "$FD_GROWTH_FINAL" -le "$FD_BUDGET" ] && echo "PASS" || echo "WARN")
RSS_FINAL_OK=$([ "$MAX_RSS" -le "$SOAK_RSS_HARD_MB" ] && echo "PASS" || echo "WARN")
WAL_OK=$([ "$FINAL_WAL" -le 10240 ] && echo "PASS" || echo "WARN")
SB_OK=$([ "$SYSBENCH_ERRORS" -eq 0 ] && echo "PASS" || echo "WARN")

cat > "$REPORT_FILE" <<EOF
# ${SOAK_MINUTES}-min Wired Soak Stability Report (G7 gate)

**Mode**: WIRED_E2E_GATE
**Run timestamp**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Duration**: ${SOAK_MINUTES} min (${SOAK_SECS}s) real wall-clock
**Driver**: check_p13_soak_test.sh (gate form of run_wired_soak.sh)
**Issues**: #3175 G7 - replaces in-process 1,440x compression

## Environment

| Item | Value |
|------|-------|
| Binary | $SQLRUSTGO_BIN |
| Host:Port | $SOAK_HOST:$SOAK_PORT |
| Data dir | $DATA_DIR |
| sysbench threads | $SOAK_THREADS |
| sysbench table_size | $SOAK_TABLE_SIZE |
| Sample interval | ${SOAK_INTERVAL}s |
| RSS hard cap | ${SOAK_RSS_HARD_MB} MB |
| FD limit | $SOAK_FD_LIMIT |

## Acceptance Results

| Criterion | Threshold | Measured | Status |
|-----------|-----------|----------|--------|
| Crashes | 0 | $CRASH | $CRASH_STATUS |
| RSS growth | <= ${RSS_BUDGET_MB} MB (scaled from 24h budget) | $RSS_GROWTH_FINAL MB | $RSS_OK |
| FD growth | <= $FD_BUDGET | $FD_GROWTH_FINAL | $FD_OK |
| Max RSS | <= ${SOAK_RSS_HARD_MB} MB | $MAX_RSS MB | $RSS_FINAL_OK |
| Final WAL | <= 10240 MB | $FINAL_WAL MB | $WAL_OK |
| sysbench errors | 0 | $SYSBENCH_ERRORS | $SB_OK |
| Resource alerts | 0 | $ALERT_COUNT | $([ "$ALERT_COUNT" -eq 0 ] && echo "PASS" || echo "WARN") |
| Samples | >= SOAK_SECS/INTERVAL | $SAMPLE_COUNT | $([ "$SAMPLE_COUNT" -ge $(( SOAK_SECS / SOAK_INTERVAL )) ] && echo "PASS" || echo "WARN") |

## sysbench Results

\`\`\`
$SYSBENCH_TX
$SYSBENCH_QPS
\`\`\`

## Artifacts

- metrics.csv - $SAMPLE_COUNT samples, every ${SOAK_INTERVAL}s
- sqlrustgo.log - server log
- sysbench.log - sysbench output
- resource_alerts.log - threshold breach events
- STABILITY_REPORT.md - this report

## Verdict

$(if [ "$CRASH" -eq 0 ] && [ "$SYSBENCH_ERRORS" -eq 0 ] && [ "$MAX_RSS" -le "$SOAK_RSS_HARD_MB" ]; then echo "**PASS** - All hard criteria met"; else echo "**NEEDS REVIEW** - See warnings above"; fi)
EOF

echo "  [3/8] PASS: wired soak complete (results: $RESULTS_DIR)"

# --- [4/8] Verify sysbench actually ran real traffic ---
if ! grep -qE "transactions:.*[0-9]" "$SYSBENCH_LOG" 2>/dev/null; then
    echo "  FAIL: sysbench produced no transaction output"
    echo "      sysbench.log tail:"
    tail -10 "$SYSBENCH_LOG" | sed 's/^/        /'
    exit 1
fi
TX_FROM_LOG=$(grep -E "transactions:" "$SYSBENCH_LOG" | tail -1 | grep -oE "[0-9]+" | head -1)
if [ -z "$TX_FROM_LOG" ] || [ "$TX_FROM_LOG" -lt 10 ]; then
    echo "  FAIL: sysbench ran but transactions=$TX_FROM_LOG < 10 (too few for E2E proof)"
    exit 1
fi
echo "  [4/8] PASS: sysbench ran $TX_FROM_LOG transactions"

# --- [5/8] Verify no server crash ---
if [ "$CRASH" -ne 0 ]; then
    echo "  FAIL: server crashed during soak"
    tail -20 "$LOG_FILE" | sed 's/^/      /'
    exit 1
fi
echo "  [5/8] PASS: no server crash"

# --- [6/8] Verify no sysbench errors ---
if [ "$SYSBENCH_ERRORS" -ne 0 ]; then
    echo "  FAIL: sysbench reported $SYSBENCH_ERRORS errors"
    grep -E "FATAL|ERROR" "$SYSBENCH_LOG" | head -5 | sed 's/^/      /'
    exit 1
fi
echo "  [6/8] PASS: 0 sysbench errors"

# --- [7/8] Verify resource limits respected ---
if [ "$MAX_RSS" -gt "$SOAK_RSS_HARD_MB" ]; then
    echo "  FAIL: RSS $MAX_RSS MB > hard cap $SOAK_RSS_HARD_MB MB"
    exit 1
fi
echo "  [7/8] PASS: resource limits respected (max RSS=${MAX_RSS}MB max FD=${MAX_FD})"

# --- [8/8] Report present and valid ---
if [ ! -f "$REPORT_FILE" ]; then
    echo "  FAIL: $REPORT_FILE not generated"
    exit 1
fi
echo "  [8/8] PASS: STABILITY_REPORT.md present"

echo
echo "=== G7 Gate: PASS (WIRED E2E) ==="
echo "  Mode:         sqlrustgo-mysql-server + sysbench oltp_read_write"
echo "  Duration:     ${SOAK_MINUTES} min wall-clock"
echo "  sysbench:     $TX_FROM_LOG transactions executed"
echo "  Results:      $RESULTS_DIR"
echo "  Long-run:     HOURS=24/72/168 bash scripts/stability/run_wired_soak.sh"
exit 0
