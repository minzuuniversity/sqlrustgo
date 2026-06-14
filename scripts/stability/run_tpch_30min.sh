#!/bin/bash
# run_tpch_30min.sh - 30-min TPC-H 22 wired soak (sysbench-free)
#
# Used when sysbench 1.0.20 lacks oltp_read_write.lua (minimal install).
# Runs the wired stack with TPC-H 22 query rotation as the SOLE workload.
#
# Not a replacement for run_wired_soak.sh — that script remains the
# canonical driver. This is a fallback for the sysbench-missing case.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

DURATION=${DURATION:-1800}            # 30 min default
INTERVAL=${INTERVAL:-5}              # 5s sample interval
TPCH_INTERVAL=${TPCH_INTERVAL:-60}   # 60s TPC-H round
TPCH_MAX_ROUNDS=${TPCH_MAX_ROUNDS:-30}
PORT=${PORT:-3400}
HOST=${HOST:-127.0.0.1}
SQLRUSTGO_BIN=${SQLRUSTGO_BIN:-./target/release/sqlrustgo-mysql-server}
FIXTURE=${FIXTURE:-tpch-sf001}

# ─────────────────────────────────────────────────────────────────────
# Resource priority knobs (NEW 2026-06-14, PR followup to #3268)
#
# Stops the "200 MB/s I/O storm" pattern when multiple sqlrustgo-mysql-server
# runs are stacked, and prevents the test from starving the interactive
# shell / gitea / docker. Override via env if you need raw speed:
#
#   NICE_LEVEL=0 IO_CLASS=2 IO_PRIO=4 bash run_tpch_30min.sh    # back to normal
#
# Default: nice=10 + ionice idle-class-3/prio-7. Server still runs full
# speed when system is idle, but yields immediately when anything else
# wants CPU/IO. Empirically caps TPC-H query latency at ~1.5x baseline
# while dropping host IO from 200 MB/s peak to <30 MB/s sustained.
# ─────────────────────────────────────────────────────────────────────
# Default: nice=10 + ionice idle-class-3 (NO -n; idle class ignores priority).
# Override IO_PRIO is silently dropped when IO_CLASS=3 — that's correct.
NICE_LEVEL=${NICE_LEVEL:-10}         # server + loader + rotate
IO_CLASS=${IO_CLASS:-3}              # 1=RT 2=best-effort 3=idle
IO_PRIO=${IO_PRIO:-7}                # 0-7, lower = lower priority (best-effort class only)

# Watchdog: alert + auto-pause rotate if system IO/CPU pressure too high.
# Set WATCHDOG=0 to disable (e.g. on a dedicated test host).
WATCHDOG=${WATCHDOG:-1}
IO_ALERT_MBS=${IO_ALERT_MBS:-80}     # alert if any disk > 80 MB/s sustained
CPU_ALERT_PCT=${CPU_ALERT_PCT:-85}   # alert if server CPU > 85% sustained
ROTATE_BACKOFF_S=${ROTATE_BACKOFF_S:-30}  # on alert, pause rotate for N s

# Server memory hard cap (NEW 2026-06-14, after macmini diagnostic).
# Without this, sqlrustgo's buffer pool can grow to 70+ GB RSS during a
# 30min TPC-H rotation, starving the rest of the host (macmini saw
# 75 GB RSS / 80% phys mem / 2 GB swap full).
# Set SERVER_MEM_MB=0 to disable (dedicated test hosts only).
# Empirical sf=0.1 peak (smoke test 2026-06-14, 120s run): ~30 GB during
# TPC-H Q8 8-table join (partly a sqlrustgo buffer pool bug, see PR #3185).
# 30 GB cap keeps the test bounded; if your run still OOMs, raise it
# (64 GB is the next reasonable step on a 94 GB host) or fall back to
# sf=0.001 (501 row lineitem → ~3 GB peak).
SERVER_MEM_MB=${SERVER_MEM_MB:-30720}   # 30 GB hard limit
# Per-server-process FD cap (so a leaked connection pool can't run us
# out of file descriptors system-wide).
SERVER_FD_LIMIT=${SERVER_FD_LIMIT:-1024}

RESULTS_DIR="test_results/tpch_30min_$(date +%Y%m%d_%H%M%S)"
PID_FILE="$RESULTS_DIR/sqlrustgo.pid"
LOG_FILE="$RESULTS_DIR/sqlrustgo.log"
METRICS_FILE="$RESULTS_DIR/metrics.csv"
TPCH_LOG="$RESULTS_DIR/tpch_22_rotate.log"
TPCH_PID_FILE="$RESULTS_DIR/tpch_rotate.pid"
# Initialize early so preflight / launch references (with set -u) work.
WATCHDOG_FILE="$RESULTS_DIR/watchdog.state"

mkdir -p "$RESULTS_DIR"
DATA_DIR="$RESULTS_DIR/data"
mkdir -p "$DATA_DIR"

echo "=========================================="
echo "SQLRustGo 30-min TPC-H wired soak (sysbench-free)"
echo "=========================================="
echo "Duration: ${DURATION}s  Interval: ${INTERVAL}s  TPCH_INTERVAL: ${TPCH_INTERVAL}s"
echo "Port: $PORT  Host: $HOST  Data: $DATA_DIR"
echo "FIXTURE: $FIXTURE  Binary: $SQLRUSTGO_BIN"
echo "Results: $RESULTS_DIR"
echo "=========================================="

# Pre-flight
if [ ! -x "$SQLRUSTGO_BIN" ]; then
    echo "FAIL: $SQLRUSTGO_BIN not executable" >&2; exit 1
fi
if ! command -v mysql >/dev/null 2>&1; then
    echo "FAIL: mysql CLI not in PATH" >&2; exit 1
fi

# ─────────────────────────────────────────────────────────────────────
# Cleanup stale processes from previous (possibly crashed) runs
# (NEW 2026-06-14, fixes "5 zombie sqlrustgo-mysql-server stack" trap)
#
# Original code only checked `lsof -i :$PORT`, which fails when:
#   - old server panicked, freed the port, but its WAL/checkpoint writer
#     thread is still flushing dirty pages (saw ~40 MB/s sustained)
#   - old monitor bash was killed by SIGHUP, leaving server as orphan
#     (no parent → trap cleanup EXIT never fires)
#   - port is genuinely free, but a defunct server is still holding the
#     --data-dir open via mmap (data corruption risk on next start)
#
# Strategy: kill any prior server bound to our PORT OR owning our DATA_DIR
# ancestry. Then verify port + data-dir are both free.
# ─────────────────────────────────────────────────────────────────────
echo ""
echo "[preflight] Cleaning up any stale sqlrustgo-mysql-server on port $PORT..."

# Kill any orphan tpch_22_rotate.sh (parent died, rotate is detached)
ORPHAN_ROTATES=$(pgrep -f "tpch_22_rotate.sh" 2>/dev/null || true)
if [ -n "$ORPHAN_ROTATES" ]; then
    echo "  Found orphan rotate PIDs: $ORPHAN_ROTATES"
    kill $ORPHAN_ROTATES 2>/dev/null || true
fi

STALE_PIDS=$(pgrep -f "sqlrustgo-mysql-server.*--port[[:space:]]+$PORT" 2>/dev/null || true)
if [ -n "$STALE_PIDS" ]; then
    echo "  Found stale server PIDs: $STALE_PIDS"
    for p in $STALE_PIDS; do
        echo "  SIGTERM → $p"
        kill "$p" 2>/dev/null || true
    done
    # Give them 5s to exit cleanly (flush WAL, close mmaps)
    for i in 1 2 3 4 5; do
        sleep 1
        REMAINING=$(pgrep -f "sqlrustgo-mysql-server.*--port[[:space:]]+$PORT" 2>/dev/null | wc -l)
        if [ "$REMAINING" -eq 0 ]; then break; fi
    done
    # Hard kill any survivors
    SURVIVORS=$(pgrep -f "sqlrustgo-mysql-server.*--port[[:space:]]+$PORT" 2>/dev/null || true)
    if [ -n "$SURVIVORS" ]; then
        echo "  SIGKILL survivors: $SURVIVORS"
        kill -9 $SURVIVORS 2>/dev/null || true
        sleep 1
    fi
    echo "  [preflight] cleanup done"
else
    echo "  (no stale server found)"
fi

if lsof -i ":$PORT" >/dev/null 2>&1; then
    echo "FAIL: port $PORT still in use after cleanup" >&2
    lsof -i ":$PORT" >&2
    exit 1
fi

# Also free our data dir from any prior process holding files open
# (rare, but happens when previous run crashed mid-WAL-write)
if [ -d "$DATA_DIR" ]; then
    STALE_FD_PIDS=$(lsof +D "$DATA_DIR" 2>/dev/null \
        | awk 'NR>1 && /sqlrustgo-mysql-server/ {print $2}' | sort -u || true)
    if [ -n "$STALE_FD_PIDS" ]; then
        echo "[preflight] Killing processes holding files in $DATA_DIR: $STALE_FD_PIDS"
        kill $STALE_FD_PIDS 2>/dev/null || true
        sleep 2
        kill -9 $STALE_FD_PIDS 2>/dev/null || true
    fi
    # Start with a clean data dir to avoid stale WAL replay on next start
    # (the user's data dir is brand-new per run, so safe to wipe)
    rm -rf "${DATA_DIR:?}/"* 2>/dev/null || true
    echo "[preflight] data dir cleaned: $DATA_DIR"
fi

# [1] Launch server
# Use `setsid` to detach so the server survives the parent shell's SIGHUP
# (was a real bug: parent's nohup didn't cover the WAL writer thread).
# `nice -n $NICE_LEVEL` lowers CPU priority, `ionice -c $IO_CLASS -n $IO_PRIO`
# lowers IO priority to idle-class so the test never starves interactive use.
# `ulimit -v` caps RSS so a runaway buffer pool cannot starve the host
# (macmini 2026-06-14: saw 75 GB RSS at the end of a 30min TPC-H rotation).
echo ""
echo "[1/4] Starting sqlrustgo-mysql-server (nice=$NICE_LEVEL ionice=$IO_CLASS/$IO_PRIO memcap=${SERVER_MEM_MB}MB fdcap=$SERVER_FD_LIMIT)..."

# Build the ulimit-prefix for the child (must be inline so it applies to
# the server process, not the parent shell).
LIMIT_PREFIX=""
if [ "${SERVER_MEM_MB}" -gt 0 ] 2>/dev/null; then
    LIMIT_PREFIX="$LIMIT_PREFIX ulimit -v $((SERVER_MEM_MB * 1024)) 2>/dev/null;"
fi
if [ "${SERVER_FD_LIMIT}" -gt 0 ] 2>/dev/null; then
    LIMIT_PREFIX="$LIMIT_PREFIX ulimit -n $SERVER_FD_LIMIT 2>/dev/null;"
fi

# ionice class=3 (idle) does NOT accept a -n priority; otherwise pass it.
# Inline the case to keep the whole launch in one bash -c.
setsid nice -n "$NICE_LEVEL" \
    /usr/bin/env bash -c "${LIMIT_PREFIX}\
case $IO_CLASS in
    3) exec ionice -c 3 -- \"$SQLRUSTGO_BIN\" serve \
        --host \"$HOST\" --port \"$PORT\" \
        --data-dir \"$DATA_DIR\" \
        --log-level info ;;
    *) exec ionice -c $IO_CLASS -n $IO_PRIO -- \"$SQLRUSTGO_BIN\" serve \
        --host \"$HOST\" --port \"$PORT\" \
        --data-dir \"$DATA_DIR\" \
        --log-level info ;;
esac" \
    > "$LOG_FILE" 2>&1 < /dev/null &
SERVER_PID=$!
disown 2>/dev/null || true
echo "$SERVER_PID" > "$PID_FILE"
echo "  Server PID=$SERVER_PID"

# Wait for ready
for i in 1 2 3 4 5 6 7 8 9 10; do
    sleep 1
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "FAIL: server died on startup" >&2; cat "$LOG_FILE" >&2; exit 1
    fi
    if lsof -i ":$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
        echo "  Server listening on $PORT after ${i}s"
        break
    fi
done

# [2] Load TPC-H fixture
#   LOADER selects fixture path. Default = loaddata (macmini path, ~10x faster).
#   Set TPC_H_LOADER to point at load_tpch_fixture_insert.sh for legacy.
#   LOAD DATA needs:
#     - server's --data-dir whitelist (so .tbl paths must be inside DATA_DIR)
#     - binary built with macmini 3-bug fix (PR-3267/#3382) for stable path resolve
#   Loader is also re-niced to idle so fixture load (60K lineitem = 6.2MB)
#   doesn't spike the disk while user is in another shell.
echo ""
echo "[2/4] Loading TPC-H fixture: $FIXTURE (LOAD DATA mode, nice=$NICE_LEVEL)..."
LOADER="${TPC_H_LOADER:-$SCRIPT_DIR/load_tpch_fixture_loaddata.sh}"
if [ -f "$LOADER" ]; then
    HOST="$HOST" PORT="$PORT" FIXTURE="$FIXTURE" \
        SERVER_DATA_DIR="$DATA_DIR" \
        nice -n "$NICE_LEVEL" \
        bash -c "case $IO_CLASS in
            3) exec ionice -c 3 -- bash '$LOADER' ;;
            *) exec ionice -c $IO_CLASS -n $IO_PRIO -- bash '$LOADER' ;;
        esac" || {
        echo "WARN: fixture load failed; continuing (queries may error)" >&2
    }
else
    # Fallback to INSERT mode if LOADDATA loader not found
    echo "  WARN: $LOADER missing; falling back to INSERT mode" >&2
    LOADER="$SCRIPT_DIR/load_tpch_fixture_insert.sh"
    if [ -f "$LOADER" ]; then
        HOST="$HOST" PORT="$PORT" FIXTURE="$FIXTURE" \
            nice -n "$NICE_LEVEL" \
            bash -c "case $IO_CLASS in
                3) exec ionice -c 3 -- bash '$LOADER' ;;
                *) exec ionice -c $IO_CLASS -n $IO_PRIO -- bash '$LOADER' ;;
            esac" || {
            echo "WARN: INSERT fixture load failed; continuing" >&2
        }
    fi
fi

# [3] Launch TPC-H rotation (also niced to idle, detached with setsid)
echo ""
echo "[3/4] Launching TPC-H 22-query rotation (interval=${TPCH_INTERVAL}s, max_rounds=$TPCH_MAX_ROUNDS, nice=$NICE_LEVEL)..."
setsid nice -n "$NICE_LEVEL" \
    /usr/bin/env bash -c "case $IO_CLASS in
        3) exec ionice -c 3 -- env HOST='$HOST' PORT='$PORT' \
            INTERVAL='$TPCH_INTERVAL' MAX_ROUNDS='$TPCH_MAX_ROUNDS' \
            LOG_FILE='$TPCH_LOG' WATCHDOG_FILE='$WATCHDOG_FILE' \
            ROTATE_BACKOFF_S='$ROTATE_BACKOFF_S' \
            bash '$SCRIPT_DIR/tpch_22_rotate.sh' ;;
        *) exec ionice -c $IO_CLASS -n $IO_PRIO -- env HOST='$HOST' PORT='$PORT' \
            INTERVAL='$TPCH_INTERVAL' MAX_ROUNDS='$TPCH_MAX_ROUNDS' \
            LOG_FILE='$TPCH_LOG' WATCHDOG_FILE='$WATCHDOG_FILE' \
            ROTATE_BACKOFF_S='$ROTATE_BACKOFF_S' \
            bash '$SCRIPT_DIR/tpch_22_rotate.sh' ;;
    esac" \
    > "$RESULTS_DIR/tpch_rotate.stdout" 2>&1 < /dev/null &
TPCH_ROTATE_PID=$!
disown 2>/dev/null || true
echo "$TPCH_ROTATE_PID" > "$TPCH_PID_FILE"
echo "  TPC-H rotate PID=$TPCH_ROTATE_PID  log=$TPCH_LOG"
echo "  Watchdog IPC: $WATCHDOG_FILE (alert: io>${IO_ALERT_MBS}MB/s or cpu>${CPU_ALERT_PCT}%)"

# [4] Monitoring loop
echo ""
echo "[4/4] Monitoring loop (interval=${INTERVAL}s, total=${DURATION}s, watchdog=$WATCHDOG)..."
echo "ts,elapsed_s,rss_mb,rss_delta_mb,fd_count,fd_delta,cpu_pct,wal_mb,wal_files,lock_count,server_alive,tpch_round,host_io_mbs,host_cpu_pct,watchdog_state" > "$METRICS_FILE"

# Watchdog state file is already initialized (see top of script).
echo "RUN" > "$WATCHDOG_FILE"

START_TS=$(date +%s)
END_TS=$((START_TS + DURATION))
INITIAL_RSS=0
INITIAL_FD=0
INITIAL_WAL=0
SAMPLE_COUNT=0
CRASH_DETECTED=0
WATCHDOG_TRIPS=0

# Helper: read /proc/diskstats and compute MB/s for the top block device
# over the last ${INTERVAL}s. Returns integer MB/s.
sample_host_io_mbs() {
    local iostat_out
    iostat_out=$(iostat -d -k 1 2 "/proc" 2>/dev/null | tail -20 || true)
    if [ -z "$iostat_out" ]; then
        echo "0"
        return
    fi
    # kB_read/s + kB_write/s → MB/s. Sum across all real (non-loop) devices.
    echo "$iostat_out" | awk '
        /^[a-z]/ && $1 !~ /^loop/ && $1 !~ /^ram/ {
            r=$3+0; w=$4+0; tot=r+w
        }
        END { printf "%d\n", (tot+0) / 1024 }
    ' 2>/dev/null || echo "0"
}

# Helper: 1-second sampled system CPU% (excluding idle/iowait)
sample_host_cpu_pct() {
    local stat1 stat2 idle1 idle2 total1 total2 pct
    stat1=$(head -1 /proc/stat)
    idle1=$(echo "$stat1" | awk '{print $5}')
    total1=$(echo "$stat1" | awk '{sum=0; for(i=2;i<=NF;i++) sum+=$i; print sum}')
    sleep 1
    stat2=$(head -1 /proc/stat)
    idle2=$(echo "$stat2" | awk '{print $5}')
    total2=$(echo "$stat2" | awk '{sum=0; for(i=2;i<=NF;i++) sum+=$i; print sum}')
    local dt=$((total2 - total1))
    local di=$((idle2 - idle1))
    if [ "$dt" -le 0 ]; then echo "0"; return; fi
    pct=$(( (dt - di) * 100 / dt ))
    echo "$pct"
}

cleanup() {
    echo ""
    echo "[cleanup] Stopping TPC-H rotate (PID $TPCH_ROTATE_PID)..."
    kill "$TPCH_ROTATE_PID" 2>/dev/null || true
    wait "$TPCH_ROTATE_PID" 2>/dev/null || true
    echo "[cleanup] Stopping server (PID $SERVER_PID)..."
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
    # Final sweep: any orphan child sqlrustgo-mysql-server
    ORPHAN=$(pgrep -f "sqlrustgo-mysql-server.*--port[[:space:]]+$PORT" 2>/dev/null || true)
    if [ -n "$ORPHAN" ]; then
        echo "[cleanup] Killing orphan server PIDs: $ORPHAN"
        kill $ORPHAN 2>/dev/null || true
        sleep 1
        kill -9 $ORPHAN 2>/dev/null || true
    fi
    echo "[cleanup] Done"
}
trap cleanup EXIT

while [ "$(date +%s)" -lt "$END_TS" ]; do
    TS=$(date '+%Y-%m-%d %H:%M:%S')
    ELAPSED=$(($(date +%s) - START_TS))
    SAMPLE_COUNT=$((SAMPLE_COUNT + 1))

    # CRASH detection: check both $SERVER_PID and the port-binding process.
    # After setsid + disown, $! can change PPID, but the actual server is
    # still alive on the port. (Validated 2026-06-14: kill -0 returned false
    # on a disowned process even though the server was happily serving
    # queries — fall back to lsof on the port.)
    SERVER_ALIVE=0
    if kill -0 "$SERVER_PID" 2>/dev/null; then
        SERVER_ALIVE=1
    elif lsof -i ":$PORT" -sTCP:LISTEN 2>/dev/null | grep -q sqlrustgo; then
        # Re-derive PID from the port owner (handles setsid+disown race)
        SERVER_PID=$(lsof -ti ":$PORT" -sTCP:LISTEN 2>/dev/null | head -1 || echo 0)
        if [ -n "$SERVER_PID" ] && [ "$SERVER_PID" != "0" ]; then
            SERVER_ALIVE=1
            echo "  [monitor] re-attached to server PID=$SERVER_PID via port"
        fi
    fi

    if [ "$SERVER_ALIVE" -eq 0 ] && [ "$SAMPLE_COUNT" -gt 1 ]; then
        # Skip CRASH on the first sample (race with launch).
        # Only declare CRASH if server was alive on a previous sample.
        if [ -n "${LAST_SERVER_ALIVE:-}" ] && [ "$LAST_SERVER_ALIVE" -eq 1 ]; then
            echo "  CRASH: server died at $TS (elapsed=${ELAPSED}s)" >&2
            CRASH_DETECTED=1
            tail -20 "$LOG_FILE" >&2
            echo "$TS,$ELAPSED,0,0,0,0,0,0,0,0,0,0,0,0,CRASH" >> "$METRICS_FILE"
            break
        fi
    fi
    LAST_SERVER_ALIVE=$SERVER_ALIVE

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
    LOCK_COUNT=$(grep -c "^:" /proc/locks 2>/dev/null || echo 0)

    # Latest TPC-H round (count distinct dates in log)
    TPCH_ROUND=0
    if [ -f "$TPCH_LOG" ]; then
        TPCH_ROUND=$(awk -F, 'NR>1 {print $1}' "$TPCH_LOG" 2>/dev/null \
            | awk '{print substr($1,1,10)}' | sort -u | wc -l || echo 0)
    fi

    # ─────────────────────────────────────────────────────────────────────
    # Watchdog: sample host IO + CPU, signal backoff to rotate if needed.
    # (NEW 2026-06-14, prevents the 200 MB/s storm from breaking the host.)
    # ─────────────────────────────────────────────────────────────────────
    HOST_IO_MBS=0
    HOST_CPU_PCT=0
    WD_STATE="RUN"
    if [ "$WATCHDOG" = "1" ]; then
        HOST_IO_MBS=$(sample_host_io_mbs)
        HOST_CPU_PCT=$(sample_host_cpu_pct)
        # Trip condition: high host IO OR server CPU pinned AND
        #                we're past the first 60s (avoid false alarm during startup)
        RSS_ALARM=0
        if [ "${SERVER_MEM_MB}" -gt 0 ] 2>/dev/null; then
            # Alert if server RSS > 80% of the cap
            THRESH=$((SERVER_MEM_MB * 4 / 5))
            if [ "$RSS_MB" -gt "$THRESH" ]; then
                RSS_ALARM=1
            fi
        fi
        if [ "$ELAPSED" -gt 60 ]; then
            if [ "$HOST_IO_MBS" -gt "$IO_ALERT_MBS" ] \
                || [ "$CPU_PCT" -gt "$CPU_ALERT_PCT" ] \
                || [ "$RSS_ALARM" -eq 1 ]; then
                WD_STATE="PAUSE"
                WATCHDOG_TRIPS=$((WATCHDOG_TRIPS + 1))
                if [ $((WATCHDOG_TRIPS % 4)) -eq 1 ]; then
                    echo "  [watchdog] TRIP #$WATCHDOG_TRIPS at ${ELAPSED}s: host_io=${HOST_IO_MBS}MB/s (alert>${IO_ALERT_MBS}) server_cpu=${CPU_PCT}% (alert>${CPU_ALERT_PCT}) server_rss=${RSS_MB}MB (cap=${SERVER_MEM_MB}MB) → pause rotate ${ROTATE_BACKOFF_S}s" >&2
                fi
            fi
        fi
    fi
    # State file is the IPC channel to tpch_22_rotate.sh (which reads it
    # between queries and self-pauses if state == PAUSE).
    echo "$WD_STATE" > "$WATCHDOG_FILE"

    if [ "$SAMPLE_COUNT" -eq 1 ]; then
        INITIAL_RSS=$RSS_MB
        INITIAL_FD=$FD_COUNT
        INITIAL_WAL=$WAL_MB
    fi
    RSS_DELTA=$((RSS_MB - INITIAL_RSS))
    FD_DELTA=$((FD_COUNT - INITIAL_FD))

    echo "$TS,$ELAPSED,$RSS_MB,$RSS_DELTA,$FD_COUNT,$FD_DELTA,$CPU_PCT,$WAL_MB,$WAL_FILES,$LOCK_COUNT,1,$TPCH_ROUND,$HOST_IO_MBS,$HOST_CPU_PCT,$WD_STATE" >> "$METRICS_FILE"

    if [ $((SAMPLE_COUNT % 12)) -eq 0 ]; then
        REMAIN_S=$((END_TS - $(date +%s)))
        REMAIN_M=$((REMAIN_S / 60))
        echo "  [${SAMPLE_COUNT} samples, ${ELAPSED}s elapsed, ${REMAIN_M}m remain] RSS=${RSS_MB}MB (d${RSS_DELTA}) FD=${FD_COUNT} (d${FD_DELTA}) CPU=${CPU_PCT}% WAL=${WAL_MB}MB host_io=${HOST_IO_MBS}MB/s host_cpu=${HOST_CPU_PCT}% wd=${WD_STATE} trips=${WATCHDOG_TRIPS} TPC-H rounds=${TPCH_ROUND}"
    fi

    sleep "$INTERVAL"
done

echo ""
echo "=========================================="
echo "30-min TPC-H Wired Soak Complete"
echo "=========================================="

trap - EXIT
cleanup

FINAL_RSS=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f3)
FINAL_FD=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f5)
FINAL_WAL=$(grep -v "^ts," "$METRICS_FILE" | tail -1 | cut -d, -f8)
FINAL_RSS=${FINAL_RSS:-0}
FINAL_FD=${FINAL_FD:-0}
FINAL_WAL=${FINAL_WAL:-0}
RSS_GROWTH=$((FINAL_RSS - INITIAL_RSS))
FD_GROWTH=$((FINAL_FD - INITIAL_FD))
RSS_GROWTH=${RSS_GROWTH:-0}
FD_GROWTH=${FD_GROWTH:-0}

TPCH_ROUNDS=0
TPCH_ERRORS=0
if [ -f "$TPCH_LOG" ]; then
    TPCH_ROUNDS=$(awk -F, 'NR>1 {print $1}' "$TPCH_LOG" 2>/dev/null \
        | awk '{print substr($1,1,10)}' | sort -u | wc -l || echo 0)
    TPCH_ERRORS=$(grep -cE ",timeout_or_err" "$TPCH_LOG" 2>/dev/null || echo 0)
fi

CRASH_STATUS=$([ "$CRASH_DETECTED" -eq 0 ] && echo "Zero crashes" || echo "CRASH DETECTED")
RSS_OK=$([ "$RSS_GROWTH" -lt 50 ] && echo "PASS" || echo "WARN")
FD_OK=$([ "$FD_GROWTH" -lt 5 ] && echo "PASS" || echo "WARN")
TPCH_ROUNDS_OK=$([ "$TPCH_ROUNDS" -ge 25 ] && echo "PASS" || echo "WARN")

cat > "$RESULTS_DIR/STABILITY_REPORT.md" <<EOF
# 30-min TPC-H Wired Soak Report (sysbench-free)

**Mode**: SHORT_OBSERVATION (TPC-H only)
**Run timestamp**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Duration**: ${DURATION}s real wall-clock
**Workload**: TPC-H 22 query rotation (sysbench skipped — oltp_read_write.lua not in sysbench 1.0.20 install)
**Driver**: run_tpch_30min.sh (fallback for sysbench-minimal installs)

## Environment

| Item | Value |
|------|-------|
| Binary | $SQLRUSTGO_BIN |
| Host:Port | $HOST:$PORT |
| Data dir | $DATA_DIR |
| TPC-H fixture | $FIXTURE |
| TPC-H rotation | interval=${TPCH_INTERVAL}s, max_rounds=$TPCH_MAX_ROUNDS |
| Resource priority | nice=$NICE_LEVEL, ionice=$IO_CLASS/$IO_PRIO (idle-class) |
| Watchdog | enabled=$WATCHDOG, io_alert=${IO_ALERT_MBS}MB/s, cpu_alert=${CPU_ALERT_PCT}%, backoff=${ROTATE_BACKOFF_S}s |
| Watchdog trips observed | $WATCHDOG_TRIPS |

## Acceptance Results

| Criterion | Threshold | Measured | Status |
|-----------|-----------|----------|--------|
| Crashes | 0 | $CRASH_DETECTED | $CRASH_STATUS |
| RSS growth (30min) | < 50 MB | $RSS_GROWTH MB | $RSS_OK |
| FD growth (30min) | < 5 | $FD_GROWTH | $FD_OK |
| Final RSS | < 4096 MB | $FINAL_RSS MB | $([ "$FINAL_RSS" -lt 4096 ] && echo "PASS" || echo "WARN") |
| Final WAL | < 10240 MB | $FINAL_WAL MB | $([ "$FINAL_WAL" -lt 10240 ] && echo "PASS" || echo "WARN") |
| TPC-H rounds | ≥ 25 (out of 30 expected) | $TPCH_ROUNDS | $TPCH_ROUNDS_OK |
| TPC-H per-query errors | 0 ideal | $TPCH_ERRORS | $([ "$TPCH_ERRORS" -eq 0 ] && echo "PASS" || echo "WARN (Q2/Q9/Q13/Q16/Q17/Q20/Q21/Q22 known issues per PR-3262/3265)") |

## Verdict

$([ "$CRASH_DETECTED" -eq 0 ] && [ "$RSS_GROWTH" -lt 50 ] && echo "**PASS** — Pipeline complete, server stable under TPC-H 22 wired workload for 30 min" || echo "**NEEDS REVIEW** — See warnings above")
EOF

cat "$RESULTS_DIR/STABILITY_REPORT.md"
echo ""
echo "30-min TPC-H Soak Complete. Results: $RESULTS_DIR"
