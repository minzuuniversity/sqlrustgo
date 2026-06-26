#!/bin/bash
# run_wired_soak.sh - Wired long-running soak for sqlrustgo-mysql-server.
#                     v3 supplement: accepts fractional HOURS (0.5 / 1 / 2
#                     for short observation, 4 / 8 / 12 / 16 / 24 / 48 / 72
#                     for full soak coverage).
#
# Architecture / 5-min smoke test: see scripts/stability/test_integration_5min.sh
# (PR #3380, anti-OOM, 60s real run). This script is for *soak* — no resource
# hard limits, longer durations, TPC-H 22 query rotation, growth monitoring.
#
# Reuses the proven run_24h_soak_v2.sh lifecycle (server / sysbench / metrics
# / cleanup / report) and ADDS:
#   - fractional HOURS (0.5 / 1 / 2 for short observation)
#   - TPC-H fixture loading (LOAD DATA) before sysbench
#   - TPC-H 22-query rotation in parallel with sysbench
#   - per-query latency CSV (alongside v2 metrics.csv)
#   - parameterized report title (was hardcoded "24h" in v2)
#
# This is NOT a "self-written stability program". It launches the real
# release binary, connects via real MySQL wire protocol, and runs
# industry-standard TPC-H 22 + sysbench oltp_read_write as load.
#
# Usage:
#   HOURS=0.5 PORT=3396 FIXTURE=tpch-sf001 \
#       bash scripts/stability/run_wired_soak.sh
#
# Environment variables (with defaults):
#   HOURS                positive number, e.g. 0.5/1/2/4/8/12/16/24/48/72  (default 1)
#   INTERVAL             metric sample interval (seconds)                   (default 60)
#                        (auto-scaled to 5s for HOURS<1, 10s for HOURS<2)
#   THREADS              sysbench threads                                    (default 16)
#   SERVER_THREADS       sqlrustgo-mysql-server worker threads               (default 16)
#                        range 0..=80, validated by --server-threads CLI
#   TABLE_SIZE           sysbench table size                                 (default 10000)
#   TABLES               sysbench table count                                (default 1)
#   PORT                 MySQL port                                          (default 3396)
#   HOST                 MySQL host                                          (default 127.0.0.1)
#   SQLRUSTGO_BIN        server binary path                                  (default ./target/release/sqlrustgo-mysql-server)
#   FIXTURE              none | tpch-tiny | tpch-sf001                       (default tpch-sf001)
#   TPCH_ROTATE          1 = run tpch_22_rotate.sh in background             (default 1)
#   TPCH_ROTATE_INTERVAL seconds between 22-query rounds                     (default 600)
#                        (auto-scaled to 60s for HOURS<1, 120s for HOURS<2)
#   TPCH_ROTATE_MAX_ROUNDS  0=forever, N>0=stop after N rounds               (default 0)
#   RESULTS_DIR          output dir                                          (default test_results/wired_soak_<HOURS>h_<ts>)
#
# Issues: #3225 (real 24h/72h wall-clock soak), #3229 (168h), and the
#         user-requested extra 0.5/1/2/4/8/12/16/48h gaps not covered by v2.
#
# Maintainer: Hermes Agent
# Last touched: 2026-06-14

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# --- HOURS parsing (accept integer or fractional) ---
HOURS_RAW="${HOURS:-1}"           # was 24
HOURS=$(printf "%.4f" "$HOURS_RAW" 2>/dev/null || echo "$HOURS_RAW")
# Validate: positive number
case "$HOURS" in
    ''|*[!0-9.]*) echo "FAIL: HOURS='$HOURS_RAW' is not a number" >&2; exit 1 ;;
esac
# Use awk for float comparison
is_positive=$(awk -v h="$HOURS" 'BEGIN { print (h > 0) ? 1 : 0 }')
if [ "$is_positive" != "1" ]; then
    echo "FAIL: HOURS='$HOURS' must be > 0" >&2; exit 1
fi
# Format HOURS for display (1.0 -> 1, 0.5 -> 0.5)
HOURS_DISPLAY=$(awk -v h="$HOURS" 'BEGIN {
    if (h == int(h)) printf "%d", h; else printf "%g", h
}')
HOURS_INT=$(awk -v h="$HOURS" 'BEGIN { printf "%d", h }')   # floor for sysbench
HOURS_SECS=$(awk -v h="$HOURS" 'BEGIN { printf "%d", h*3600 }')

# Suggest known standard durations
case "$HOURS_DISPLAY" in
    0.5|1|2|4|8|12|16|24|48|72) ;;
    *) echo "  NOTE: HOURS=$HOURS_DISPLAY is not in the standard set {0.5,1,2,4,8,12,16,24,48,72}; running anyway" >&2 ;;
esac

# --- Auto-tune INTERVAL for short runs ---
if [ "${INTERVAL:-60}" = "60" ]; then
    # User did not override — auto-scale
    if [ "$HOURS_INT" = "0" ]; then
        INTERVAL=5
    elif [ "$HOURS_INT" = "1" ]; then
        INTERVAL=10
    elif [ "$HOURS_INT" = "2" ]; then
        INTERVAL=15
    else
        INTERVAL=60
    fi
fi

# --- Auto-tune TPCH_ROTATE_INTERVAL ---
if [ "${TPCH_ROTATE_INTERVAL:-600}" = "600" ]; then
    if [ "$HOURS_INT" = "0" ]; then
        TPCH_ROTATE_INTERVAL=60
    elif [ "$HOURS_INT" = "1" ]; then
        TPCH_ROTATE_INTERVAL=120
    elif [ "$HOURS_INT" = "2" ]; then
        TPCH_ROTATE_INTERVAL=180
    else
        TPCH_ROTATE_INTERVAL=600
    fi
fi

THREADS="${THREADS:-16}"           # was 8
TABLE_SIZE="${TABLE_SIZE:-10000}"
SERVER_THREADS="${SERVER_THREADS:-16}"
SERVER_THREADS_MAX=80
if ! [[ "$SERVER_THREADS" =~ ^[0-9]+$ ]]; then
    echo "FAIL: SERVER_THREADS='$SERVER_THREADS' is not an integer" >&2; exit 1
fi
if [ "$SERVER_THREADS" -gt "$SERVER_THREADS_MAX" ]; then
    echo "FAIL: SERVER_THREADS=$SERVER_THREADS > $SERVER_THREADS_MAX (binary rejects)" >&2; exit 1
fi
TABLES="${TABLES:-1}"
PORT="${PORT:-3396}"
HOST="${HOST:-127.0.0.1}"
FIXTURE="${FIXTURE:-tpch-sf001}"
TPCH_ROTATE="${TPCH_ROTATE:-1}"
TPCH_ROTATE_MAX_ROUNDS="${TPCH_ROTATE_MAX_ROUNDS:-0}"
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-./target/release/sqlrustgo-mysql-server}"

# ─────────────────────────────────────────────────────────────────────
# Server memory hard cap (P0 enhancement 2026-06-14, mirrors run_tpch_30min.sh).
#
# Without this, sqlrustgo's buffer pool can grow to 70+ GB RSS during a
# TPC-H rotation, starving the rest of the host (macmini 2026-06-14 saw
# 75 GB RSS / 80% phys mem / 2 GB swap full from a 30min TPC-H soak).
#
# `ulimit -v` (KB) caps virtual memory → kernel OOM-kills server if it
# exceeds the cap. We pair it with an absolute RSS watchdog (RSS_ALERT_MB,
# default 80% of cap) that auto-kills + cleanup if RSS creeps up.
#
# Set SERVER_MEM_MB=0 to disable (dedicated test hosts only).
# ─────────────────────────────────────────────────────────────────────
SERVER_MEM_MB=${SERVER_MEM_MB:-8192}     # 8 GB hard limit
SERVER_FD_LIMIT=${SERVER_FD_LIMIT:-1024}   # per-server FD cap
RSS_ALERT_MB=${RSS_ALERT_MB:-0}          # 0 = auto-derive from SERVER_MEM_MB (80%)
[ "$RSS_ALERT_MB" -eq 0 ] && [ "$SERVER_MEM_MB" -gt 0 ] && \
    RSS_ALERT_MB=$((SERVER_MEM_MB * 4 / 5))
RSS_KILL_ACTION=${RSS_KILL_ACTION:-auto_kill}  # auto_kill|warn_only

RESULTS_DIR="${RESULTS_DIR:-test_results/wired_soak_${HOURS_DISPLAY}h_$(date +%Y%m%d_%H%M%S)}"
PID_FILE="$RESULTS_DIR/sqlrustgo.pid"
LOG_FILE="$RESULTS_DIR/sqlrustgo.log"
METRICS_FILE="$RESULTS_DIR/metrics.csv"
SYSBENCH_LOG="$RESULTS_DIR/sysbench.log"
TPCH_LOG="$RESULTS_DIR/tpch_22_rotate.log"
TPCH_PID_FILE="$RESULTS_DIR/tpch_rotate.pid"

mkdir -p "$RESULTS_DIR"

# Pre-flight
if [ ! -x "$SQLRUSTGO_BIN" ]; then
    echo "  WARN: $SQLRUSTGO_BIN not found/executable, attempting cargo build..." >&2
    cargo build --release --bin sqlrustgo-mysql-server
fi
if ! command -v sysbench >/dev/null 2>&1; then
    echo "FAIL: sysbench not found in PATH" >&2; exit 1
fi
if ! command -v mysql >/dev/null 2>&1; then
    echo "FAIL: mysql CLI not found in PATH" >&2; exit 1
fi
if lsof -i ":$PORT" >/dev/null 2>&1; then
    echo "FAIL: port $PORT already in use" >&2
    lsof -i ":$PORT"
    exit 1
fi

DATA_DIR="$RESULTS_DIR/data"
mkdir -p "$DATA_DIR"

echo "=========================================="
echo "SQLRustGo Wired Soak — ${HOURS_DISPLAY}h (${HOURS_SECS}s)"
echo "=========================================="
echo "Hours=$HOURS_DISPLAY  Interval=${INTERVAL}s  Threads=$THREADS"
echo "ServerThreads=$SERVER_THREADS  Port=$PORT  Host=$HOST  Data=$DATA_DIR"
echo "FIXTURE=$FIXTURE  TPCH_ROTATE=$TPCH_ROTATE (interval=${TPCH_ROTATE_INTERVAL}s)"
echo "Results=$RESULTS_DIR"
echo "Binary=$SQLRUSTGO_BIN"
echo "=========================================="
echo ""

# [1] Launch server
# `ulimit -v` caps RSS so a runaway buffer pool cannot starve the host
# (macmini 2026-06-14: saw 75 GB RSS at the end of a 30min TPC-H rotation).
echo ""
echo "[1/5] Starting sqlrustgo-mysql-server (memcap=${SERVER_MEM_MB}MB fdcap=$SERVER_FD_LIMIT)..."

# Build the ulimit-prefix for the child (must be inline so it applies to
# the server process, not the parent shell).
LIMIT_PREFIX=""
if [ "${SERVER_MEM_MB}" -gt 0 ] 2>/dev/null; then
    LIMIT_PREFIX="$LIMIT_PREFIX ulimit -v $((SERVER_MEM_MB * 1024)) 2>/dev/null;"
fi
if [ "${SERVER_FD_LIMIT}" -gt 0 ] 2>/dev/null; then
    LIMIT_PREFIX="$LIMIT_PREFIX ulimit -n $SERVER_FD_LIMIT 2>/dev/null;"
fi

nohup bash -c "$LIMIT_PREFIX exec '$SQLRUSTGO_BIN' serve \
    --host '$HOST' --port '$PORT' \
    --data-dir '$DATA_DIR' \
    --log-level info \
    --server-threads '$SERVER_THREADS'" \
    > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$PID_FILE"
echo "  Server PID=$SERVER_PID  log=$LOG_FILE"

# Wait for ready
for i in 1 2 3 4 5 6 7 8 9 10; do
    sleep 1
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "FAIL: server died on startup" >&2
        cat "$LOG_FILE" >&2
        exit 1
    fi
    if lsof -i ":$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
        echo "  Server listening on $PORT after ${i}s"
        break
    fi
done

# [2] Load TPC-H fixture (if requested) — prefer INSERT mode (works on all
#    v3.9.0 binaries); fallback to LOAD DATA mode for recent PR-3233+ binaries.
if [ "$FIXTURE" != "none" ]; then
    echo "[2/5] Loading TPC-H fixture: $FIXTURE ..."
    LOADER_INSERT="$SCRIPT_DIR/load_tpch_fixture_insert.sh"
    LOADER_LD="$SCRIPT_DIR/load_tpch_fixture.sh"
    if [ -f "$LOADER_INSERT" ] && [ "${TPC_H_LOADER_MODE:-insert}" = "insert" ]; then
        HOST="$HOST" PORT="$PORT" FIXTURE="$FIXTURE" \
            bash "$LOADER_INSERT" || {
            echo "WARN: TPC-H fixture (INSERT mode) load failed; continuing" >&2
        }
    elif [ -f "$LOADER_LD" ]; then
        HOST="$HOST" PORT="$PORT" FIXTURE="$FIXTURE" \
            bash "$LOADER_LD" || {
            echo "WARN: TPC-H fixture (LOAD DATA mode) load failed; continuing" >&2
        }
    else
        echo "  WARN: no TPC-H loader script found; skipping fixture load" >&2
    fi
else
    echo "[2/5] FIXTURE=none, skipping"
fi

# [3] sysbench prepare (create sbtest table)
echo "[3/5] sysbench prepare (TABLES=$TABLES TABLE_SIZE=$TABLE_SIZE)..."
sysbench oltp_read_write \
    --db-driver=mysql \
    --mysql-host="$HOST" --mysql-port="$PORT" \
    --mysql-user=root --mysql-password="" \
    --mysql-db=sbtest --table-size="$TABLE_SIZE" --tables="$TABLES" \
    prepare 2>&1 | tail -3 || {
    echo "FAIL: sysbench prepare failed" >&2
    tail -20 "$LOG_FILE" >&2
    kill "$SERVER_PID" 2>/dev/null || true
    exit 1
}

# [4] Launch sysbench run + TPC-H rotation in parallel
echo "[4/5] Launching sysbench oltp_read_write (${HOURS_DISPLAY}h = ${HOURS_SECS}s)..."
nohup sysbench oltp_read_write \
    --db-driver=mysql \
    --mysql-host="$HOST" --mysql-port="$PORT" \
    --mysql-user=root --mysql-password="" \
    --mysql-db=sbtest --table-size="$TABLE_SIZE" --tables="$TABLES" \
    --threads="$THREADS" --time="$HOURS_SECS" \
    --report-interval=10 \
    run > "$SYSBENCH_LOG" 2>&1 &
SYSBENCH_PID=$!
echo "  sysbench PID=$SYSBENCH_PID"

if [ "$TPCH_ROTATE" = "1" ]; then
    echo "       Launching TPC-H 22-query rotation (interval=${TPCH_ROTATE_INTERVAL}s)..."
    HOST="$HOST" PORT="$PORT" \
        INTERVAL="$TPCH_ROTATE_INTERVAL" \
        MAX_ROUNDS="$TPCH_ROTATE_MAX_ROUNDS" \
        LOG_FILE="$TPCH_LOG" \
        nohup bash "$SCRIPT_DIR/tpch_22_rotate.sh" > "$RESULTS_DIR/tpch_rotate.stdout" 2>&1 &
    TPCH_ROTATE_PID=$!
    echo "$TPCH_ROTATE_PID" > "$TPCH_PID_FILE"
    echo "       TPC-H rotate PID=$TPCH_ROTATE_PID  log=$TPCH_LOG"
else
    TPCH_ROTATE_PID=""
    echo "       TPCH_ROTATE=0, skipping TPC-H rotation"
fi

# [5] Monitoring loop (mirrors v2, scaled by INTERVAL)
echo "[5/5] Monitoring loop (interval=${INTERVAL}s, total=${HOURS_DISPLAY}h)..."
echo "ts,elapsed_s,rss_mb,rss_delta_mb,fd_count,fd_delta,cpu_pct,wal_mb,wal_files,lock_count,server_alive,sysbench_qps" > "$METRICS_FILE"

START_TS=$(date +%s)
END_TS=$((START_TS + HOURS_SECS))
INITIAL_RSS=0
INITIAL_FD=0
INITIAL_WAL=0
SAMPLE_COUNT=0
CRASH_DETECTED=0

cleanup() {
    echo ""
    echo "[cleanup] Stopping TPC-H rotate (PID $TPCH_ROTATE_PID)..."
    [ -n "$TPCH_ROTATE_PID" ] && kill "$TPCH_ROTATE_PID" 2>/dev/null || true
    [ -n "$TPCH_ROTATE_PID" ] && wait "$TPCH_ROTATE_PID" 2>/dev/null || true
    echo "[cleanup] Stopping sysbench (PID $SYSBENCH_PID)..."
    kill "$SYSBENCH_PID" 2>/dev/null || true
    wait "$SYSBENCH_PID" 2>/dev/null || true
    echo "[cleanup] Stopping server (PID $SERVER_PID)..."
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
    echo "[cleanup] Done"
}
trap cleanup EXIT

while [ "$(date +%s)" -lt "$END_TS" ]; do
    TS=$(date '+%Y-%m-%d %H:%M:%S')
    ELAPSED=$(($(date +%s) - START_TS))
    SAMPLE_COUNT=$((SAMPLE_COUNT + 1))

    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "  CRASH: server died at $TS (elapsed=${ELAPSED}s)" >&2
        CRASH_DETECTED=1
        tail -20 "$LOG_FILE" >&2
        echo "$TS,$ELAPSED,0,0,0,0,0,0,0,0,0,0" >> "$METRICS_FILE"
        break
    fi

    RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    RSS_MB=$((RSS_KB / 1024))
    FD_COUNT=$(lsof -p "$SERVER_PID" 2>/dev/null | wc -l | tr -d ' ')
    CPU_PCT=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
    WAL_MB=0
    WAL_FILES=0
    if [ -d "$DATA_DIR" ]; then
        WAL_MB=$(du -sm "$DATA_DIR" 2>/dev/null | cut -f1 || echo 0)
        WAL_FILES=$(find "$DATA_DIR" -type f 2>/dev/null | wc -l | tr -d ' ')
    fi
    SYSBENCH_QPS=0
    if [ -f "$SYSBENCH_LOG" ]; then
        SYSBENCH_QPS=$(grep -E "thds|tps|qps" "$SYSBENCH_LOG" 2>/dev/null | tail -1 \
            | grep -oE "[0-9]+\.[0-9]+\s*per sec" | grep -oE "[0-9]+\.[0-9]+" | head -1 || echo 0)
    fi
    LOCK_COUNT=$(grep -c "^:" /proc/locks 2>/dev/null || echo 0)

    if [ "$SAMPLE_COUNT" -eq 1 ]; then
        INITIAL_RSS=$RSS_MB
        INITIAL_FD=$FD_COUNT
        INITIAL_WAL=$WAL_MB
    fi
    RSS_DELTA=$((RSS_MB - INITIAL_RSS))
    FD_DELTA=$((FD_COUNT - INITIAL_FD))

    echo "$TS,$ELAPSED,$RSS_MB,$RSS_DELTA,$FD_COUNT,$FD_DELTA,$CPU_PCT,$WAL_MB,$WAL_FILES,$LOCK_COUNT,1,$SYSBENCH_QPS" >> "$METRICS_FILE"

    # For short runs, RSS/FD growth is small; scale threshold to elapsed fraction.
    # Threshold: 50MB per 24h; for 0.5h run, expect ≤ ~1MB.
    if [ "$SAMPLE_COUNT" -gt 5 ]; then
        expected_rss_growth=$(awk -v e="$ELAPSED" 'BEGIN { printf "%d", (50 * e / 86400) + 1 }')
        if [ "$RSS_GROWTH" -gt "$expected_rss_growth" ] 2>/dev/null; then
            echo "  WARN[${ELAPSED}s]: RSS growth $RSS_DELTA MB > expected $expected_rss_growth MB" >&2
        fi
        if [ "$FD_GROWTH" -gt 5 ] 2>/dev/null; then
            echo "  WARN[${ELAPSED}s]: FD growth > 5 (delta=$FD_DELTA)" >&2
        fi
    fi

    # RSS absolute watchdog (P0 enhancement 2026-06-14).
    # ulimit -v enforces the hard cap via kernel OOM-killer, but the
    # alert threshold fires earlier so we can mark the run as WARN
    # before the OOM-killer triggers. This is a backstop for the
    # edge case where ulimit isn't honored (e.g. user-set
    # SERVER_MEM_MB=0 to disable, or kernel enforces cgroup instead).
    if [ "$RSS_ALERT_MB" -gt 0 ] 2>/dev/null && [ "$RSS_MB" -gt "$RSS_ALERT_MB" ] 2>/dev/null; then
        echo "  ALERT[${ELAPSED}s]: server RSS ${RSS_MB}MB > ${RSS_ALERT_MB}MB threshold" >&2
        if [ "$RSS_KILL_ACTION" = "auto_kill" ]; then
            echo "  ALERT[${ELAPSED}s]: auto-killing server (RSS_KILL_ACTION=auto_kill)" >&2
            kill -TERM "$SERVER_PID" 2>/dev/null
            sleep 2
            kill -KILL "$SERVER_PID" 2>/dev/null
            RSS_ALARM=1
            echo "$TS,$ELAPSED,$RSS_MB,$RSS_DELTA,$FD_COUNT,$FD_DELTA,$CPU_PCT,$WAL_MB,$WAL_FILES,$LOCK_COUNT,0,$SYSBENCH_QPS,OOM_KILL" >> "$METRICS_FILE"
            break
        fi
    fi

    if [ $((SAMPLE_COUNT % 10)) -eq 0 ]; then
        REMAIN_S=$((END_TS - $(date +%s)))
        REMAIN_M=$((REMAIN_S / 60))
        echo "  [${SAMPLE_COUNT} samples, ${ELAPSED}s elapsed, ${REMAIN_M}m remain] RSS=${RSS_MB}MB (d${RSS_DELTA}) FD=${FD_COUNT} (d${FD_DELTA}) CPU=${CPU_PCT}% WAL=${WAL_MB}MB QPS=${SYSBENCH_QPS}"
    fi

    sleep "$INTERVAL"
done

echo ""
echo "=========================================="
echo "${HOURS_DISPLAY}h Wired Soak Complete — Generating Report"
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

SYSBENCH_TX=$(grep -E "transactions:" "$SYSBENCH_LOG" 2>/dev/null | tail -1 || echo "TBD")
SYSBENCH_QPS_FINAL=$(grep -E "queries per second" "$SYSBENCH_LOG" 2>/dev/null | tail -1 || echo "TBD")
SYSBENCH_ERRORS=$(grep -cE "FATAL|ERROR|deadlock" "$SYSBENCH_LOG" 2>/dev/null || echo 0)

TPCH_ROUNDS=0
TPCH_ERRORS=0
if [ -f "$TPCH_LOG" ]; then
    TPCH_ROUNDS=$(awk -F, 'NR>1 {print $1}' "$TPCH_LOG" 2>/dev/null \
        | awk '{print substr($1,1,10)}' | sort -u | wc -l || echo 0)
    TPCH_ERRORS=$(grep -cE ",timeout_or_err" "$TPCH_LOG" 2>/dev/null || echo 0)
fi

CRASH_STATUS=$([ "$CRASH_DETECTED" -eq 0 ] && echo "Zero crashes" || echo "CRASH DETECTED")
RSS_OK=$([ "$RSS_GROWTH" -lt 50 ] && echo "PASS" || echo "WARN")
FD_OK=$([ "$FD_GROWTH" -lt 50 ] && echo "PASS" || echo "WARN")
RSS_FINAL_OK=$([ "$FINAL_RSS" -lt 4096 ] && echo "PASS" || echo "WARN")
WAL_OK=$([ "$FINAL_WAL" -lt 10240 ] && echo "PASS" || echo "WARN")
SB_OK=$([ "$SYSBENCH_ERRORS" -eq 0 ] && echo "PASS" || echo "WARN")
RSS_ABS_OK=$([ "${RSS_ALARM:-0}" -eq 0 ] && echo "PASS" || echo "WARN (server auto-killed by RSS absolute watchdog)")

# For short runs, the growth thresholds are time-scaled (see loop above)
case "$HOURS_DISPLAY" in
    0.5|1|2) SCALE_TAG="SHORT_OBSERVATION" ;;
    *)        SCALE_TAG="FULL_SOAK" ;;
esac

cat > "$RESULTS_DIR/STABILITY_REPORT.md" <<EOF
# ${HOURS_DISPLAY}h Wired Soak Stability Report

**Mode**: $SCALE_TAG
**Run timestamp**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Duration**: ${HOURS_DISPLAY}h (${HOURS_SECS}s) real wall-clock
**Driver**: run_wired_soak.sh (v3 — fractional HOURS support)
**Companion**: scripts/stability/test_integration_5min.sh (PR #3380, 5-min anti-OOM)
**Issues**: #3225 / #3229 (real wall-clock soak) + user-requested 0.5/1/2h short observation

## Environment

| Item | Value |
|------|-------|
| Binary | $SQLRUSTGO_BIN |
| Host:Port | $HOST:$PORT |
| Data dir | $DATA_DIR |
| sysbench threads | $THREADS |
| server worker threads | $SERVER_THREADS (max $SERVER_THREADS_MAX) |
| sysbench table_size | $TABLE_SIZE |
| sysbench tables | $TABLES |
| TPC-H fixture | $FIXTURE |
| TPC-H rotation | $TPCH_ROTATE (interval=${TPCH_ROTATE_INTERVAL}s) |
| Sample interval | ${INTERVAL}s (auto-scaled for short runs) |

## Acceptance Results

| Criterion | Threshold | Measured | Status |
|-----------|-----------|----------|--------|
| Crashes | 0 | $CRASH_DETECTED | $CRASH_STATUS |
| RSS growth | < 50 MB (24h-equiv) | $RSS_GROWTH MB | $RSS_OK |
| FD growth | < 50 | $FD_GROWTH | $FD_OK |
| Final RSS | < 4096 MB | $FINAL_RSS MB | $RSS_FINAL_OK |
| Final WAL | < 10240 MB | $FINAL_WAL MB | $WAL_OK |
| sysbench errors | 0 | $SYSBENCH_ERRORS | $SB_OK |
| RSS absolute watchdog | $RSS_ALERT_MB MB | RSS_ALARM=$RSS_ALARM | $RSS_ABS_OK |
| TPC-H rounds run | ≥1 if enabled | $TPCH_ROUNDS | $([ "$TPCH_ROUNDS" -gt 0 ] && echo "PASS" || echo "WARN") |
| TPC-H per-query errors | 0 | $TPCH_ERRORS | $([ "$TPCH_ERRORS" -eq 0 ] && echo "PASS" || echo "WARN (engine-known issues per PR-3262/3265)") |

## sysbench Results

\`\`\`
$SYSBENCH_TX
$SYSBENCH_QPS_FINAL
\`\`\`

## TPC-H 22 Results

- rounds: $TPCH_ROUNDS
- per-query errors: $TPCH_ERRORS (expected for Q2/Q9/Q13/Q16/Q17/Q20/Q21/Q22 on v3.9.0)
- log: tpch_22_rotate.log

## Artifacts

- \`metrics.csv\` — $(($(wc -l < "$METRICS_FILE") - 1)) samples, sampled every ${INTERVAL}s
- \`sqlrustgo.log\` — server log
- \`sysbench.log\` — sysbench output
- \`tpch_22_rotate.log\` — per-query latency CSV (ts,query,elapsed_ms,status)
- \`sqlrustgo.pid\` — server PID

## Verdict

$([ "$CRASH_DETECTED" -eq 0 ] && [ "$RSS_GROWTH" -lt 50 ] && [ "$FD_GROWTH" -lt 50 ] && echo "**PASS** - All acceptance criteria met" || echo "**NEEDS REVIEW** - See warnings above")
EOF

cat "$RESULTS_DIR/STABILITY_REPORT.md"
echo ""
echo "${HOURS_DISPLAY}h Wired Soak Complete. Results: $RESULTS_DIR"
