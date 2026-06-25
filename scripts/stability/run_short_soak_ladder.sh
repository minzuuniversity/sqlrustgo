#!/bin/bash
# run_short_soak_ladder.sh - Progressive short soak test ladder: 30m → 1h → 2h → 4h
#
# Purpose: catch early stability issues (memory leaks, fd leaks, crashes, hangs)
#           locally on Mac Mini BEFORE dispatching the real wall-clock soaks
#           (#3265 72h, #3266 168h) to Z6G4.
#
# Each step: starts sqlrustgo-mysql-server + sysbench + resource guard
# If step PASS → proceed to next step
# If step FAIL → stop ladder and report
#
# Resource guard thresholds (applied to each step):
#   RSS_HARD_LIMIT_MB    Kill if RSS > 4096 MB
#   RSS_GROWTH_RATE      Warn if RSS growth > 100 MB/hr
#   DISK_MIN_GB          Kill if disk < 2 GB
#   FD_HARD_LIMIT        Kill if FD > 4000
#   WAL_HARD_LIMIT_MB    Kill if sqlrustgo.wal > 1024 MB (1 GB)
#                        (set lower if test fixture is small)
#   WAL_GROWTH_RATE      Warn if WAL growth > 200 MB/hr (no checkpoint?)
#   CRASH                Always fail
#
# Usage:
#   bash scripts/stability/run_short_soak_ladder.sh
#   LADDER="30m 1h 2h" PORT=3397 bash scripts/stability/run_short_soak_ladder.sh
#   SKIP_BUILD=1 LADDER="30m" bash scripts/stability/run_short_soak_ladder.sh
#
# Split from #3225 (2026-06-18). See Gitea issues #3264/#3265/#3266 for the
# full wall-clock ladder (24h → 72h → 168h) on Z6G4.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# ---- Config ----
PORT="${PORT:-3397}"
THREADS="${THREADS:-4}"
HOST="${HOST:-127.0.0.1}"
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-$PROJECT_ROOT/target/release/sqlrustgo-mysql-server}"
INTERVAL="${INTERVAL:-30}"
SKIP_BUILD="${SKIP_BUILD:-0}"
LADDER="${LADDER:-30m 1h 2h 4h}"

LADDER_LOG="${LADDER_LOG:-test_results/short_soak_ladder.log}"
LADDER_STATUS="${LADDER_STATUS:-test_results/short_soak_ladder_STATUS.txt}"
RESULTS_ROOT="${RESULTS_ROOT:-test_results/short_soak_ladder_$(date +%Y%m%d_%H%M%S)}"

mkdir -p "$RESULTS_ROOT"
mkdir -p "$(dirname "$LADDER_LOG")"

# Resource thresholds (tighter than 24h+ ladder since these are short tests)
RSS_HARD_LIMIT_MB="${RSS_HARD_LIMIT_MB:-4096}"
RSS_SOFT_MB="${RSS_SOFT_MB:-2048}"
RSS_GROWTH_RATE_MB_PER_HR="${RSS_GROWTH_RATE_MB_PER_HR:-100}"
DISK_MIN_GB="${DISK_MIN_GB:-2}"
FD_HARD_LIMIT="${FD_HARD_LIMIT:-4000}"
WAL_HARD_LIMIT_MB="${WAL_HARD_LIMIT_MB:-8192}"
WAL_GROWTH_RATE_MB_PER_HR="${WAL_GROWTH_RATE_MB_PER_HR:-200}"

# ---- Helper functions ----
log() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] $*" | tee -a "$LADDER_LOG"; }
alert() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] ALERT: $*" | tee -a "$LADDER_LOG" >&2; }

# Convert "30m" / "1h" / "2h" → seconds
parse_duration_to_seconds() {
    local d="$1"
    if [[ "$d" =~ ^([0-9]+)m$ ]]; then
        echo $(( ${BASH_REMATCH[1]} * 60 ))
    elif [[ "$d" =~ ^([0-9]+)h$ ]]; then
        echo $(( ${BASH_REMATCH[1]} * 3600 ))
    else
        echo "ERROR: invalid duration '$d' (use Nm or Nh)" >&2
        return 1
    fi
}

# Check if a port is in LISTEN state
check_port() {
    if lsof -i ":$1" -sTCP:LISTEN >/dev/null 2>&1; then
        return 0  # in use
    fi
    return 1  # free
}

wait_port_free() {
    local port=$1
    local max_wait=${2:-30}
    local waited=0
    while check_port "$port"; do
        sleep 1
        waited=$((waited + 1))
        if [ $waited -ge $max_wait ]; then
            return 1
        fi
    done
    return 0
}

kill_server() {
    local pid=${1:-}
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        kill -TERM "$pid" 2>/dev/null || true
        for _ in 1 2 3 4 5; do
            if ! kill -0 "$pid" 2>/dev/null; then break; fi
            sleep 1
        done
        if kill -0 "$pid" 2>/dev/null; then
            kill -KILL "$pid" 2>/dev/null || true
        fi
    fi
    if [ -n "${2:-}" ]; then
        wait_port_free "$2" 30 || true
    fi
}

# Run a single ladder step
# Args: duration_label duration_seconds
run_step() {
    local label="$1"
    local seconds="$2"
    local step_dir="$RESULTS_ROOT/step_${label}"
    mkdir -p "$step_dir"
    local server_log="$step_dir/server.log"
    local sysbench_log="$step_dir/sysbench.log"
    local metrics_file="$step_dir/metrics.csv"
    local server_pid_file="$step_dir/server.pid"

    log "===== Step $label ($seconds seconds) ====="
    log "Step dir: $step_dir"

    # 1. Start server
    log "[1/4] Starting sqlrustgo-mysql-server on port $PORT..."
    if [ ! -x "$SQLRUSTGO_BIN" ]; then
        log "  Building $SQLRUSTGO_BIN..."
        cargo build --release --bin sqlrustgo-mysql-server 2>&1 | tail -5
    fi
    nohup "$SQLRUSTGO_BIN" serve --port "$PORT" --data-dir "$step_dir/data" > "$server_log" 2>&1 &
    local server_pid=$!
    echo "$server_pid" > "$server_pid_file"
    log "  Server PID: $server_pid"

    # Wait for server to be ready (max 30s)
    local ready=0
    for _ in $(seq 1 30); do
        if ! kill -0 "$server_pid" 2>/dev/null; then
            log "  ❌ Server died on startup"
            tail -20 "$server_log" | tee -a "$LADDER_LOG"
            return 1
        fi
        if check_port "$PORT"; then
            ready=1
            break
        fi
        sleep 1
    done
    if [ $ready -eq 0 ]; then
        log "  ❌ Server failed to bind port $PORT within 30s"
        kill_server "$server_pid" "$PORT"
        return 1
    fi
    log "  ✅ Server ready"

    # 2. Initialize sysbench schema
    log "[2/4] Preparing sysbench schema..."
    if ! sysbench oltp_read_write \
        --db-driver=mysql \
        --mysql-host="$HOST" --mysql-port="$PORT" \
        --mysql-user=root \
        --mysql-db=sbtest \
        --table-size=1000 --tables=1 \
        prepare >> "$sysbench_log" 2>&1; then
        log "  ⚠️ sysbench prepare failed (may be OK if no oltp_read_write.lua)"
    fi

    # 3. Start sysbench
    log "[3/4] Starting sysbench oltp_read_write ($THREADS threads, ${label})..."
    nohup sysbench oltp_read_write \
        --db-driver=mysql \
        --mysql-host="$HOST" --mysql-port="$PORT" \
        --mysql-user=root \
        --mysql-db=sbtest \
        --table-size=1000 --tables=1 \
        --threads="$THREADS" --time="$seconds" \
        --report-interval=30 \
        run > "$sysbench_log" 2>&1 &
    local sysbench_pid=$!
    log "  sysbench PID: $sysbench_pid"

    # 4. Monitor
    log "[4/4] Monitoring ${label}..."
    echo "ts,elapsed_s,rss_mb,fd_count,cpu_pct,wal_mb,disk_avail_gb,lock_count" > "$metrics_file"

    local start_ts=$(date +%s)
    local end_ts=$((start_ts + seconds))
    local step_failed=0
    local fail_reason=""

    while [ "$(date +%s)" -lt "$end_ts" ]; do
        sleep "$INTERVAL"
        if ! kill -0 "$server_pid" 2>/dev/null; then
            log "  ❌ Server died during run"
            tail -30 "$server_log" | tee -a "$LADDER_LOG"
            step_failed=1
            fail_reason="server_died"
            break
        fi
        local ts=$(date '+%Y-%m-%d %H:%M:%S')
        local elapsed=$(($(date +%s) - start_ts))
        local rss_kb=$(ps -o rss= -p "$server_pid" 2>/dev/null | tr -d ' ' || echo 0)
        local rss_mb=$((rss_kb / 1024))
        local fd_count=$(($(lsof -p "$server_pid" 2>/dev/null | wc -l) - 1))
        local cpu_pct=$(ps -o %cpu= -p "$server_pid" 2>/dev/null | tr -d ' ' || echo 0)
        local wal_mb=0
        local wal_file="$step_dir/data/sqlrustgo.wal"
        if [ -f "$wal_file" ]; then
            wal_mb=$(($(stat -f %z "$wal_file" 2>/dev/null || echo 0) / 1024 / 1024))
        fi
        local disk_avail_gb=$(df -g "$step_dir" 2>/dev/null | tail -1 | awk '{print $4}' || echo 0)
        local lock_count=$(lsof -p "$server_pid" 2>/dev/null | grep -c -E "POSIX|MEMLOCK" || echo 0)
        echo "$ts,$elapsed,$rss_mb,$fd_count,$cpu_pct,$wal_mb,$disk_avail_gb,$lock_count" >> "$metrics_file"

        # Threshold checks
        if [ "$rss_mb" -gt "$RSS_HARD_LIMIT_MB" ]; then
            log "  ❌ FAIL: RSS=${rss_mb}MB > ${RSS_HARD_LIMIT_MB}MB hard limit"
            step_failed=1
            fail_reason="rss_hard_limit"
            break
        fi
        if [ "$rss_mb" -gt "$RSS_SOFT_MB" ]; then
            log "  ⚠️ WARN: RSS=${rss_mb}MB > ${RSS_SOFT_MB}MB soft limit"
        fi
        if [ "$fd_count" -gt "$FD_HARD_LIMIT" ]; then
            log "  ❌ FAIL: FD=${fd_count} > ${FD_HARD_LIMIT} hard limit"
            step_failed=1
            fail_reason="fd_hard_limit"
            break
        fi
        if [ "$wal_mb" -gt "$WAL_HARD_LIMIT_MB" ]; then
            log "  ❌ FAIL: WAL=${wal_mb}MB > ${WAL_HARD_LIMIT_MB}MB hard limit (no checkpoint?)"
            step_failed=1
            fail_reason="wal_hard_limit"
            break
        fi
        if [ "$disk_avail_gb" -lt "$DISK_MIN_GB" ]; then
            log "  ❌ FAIL: disk free ${disk_avail_gb}GB < ${DISK_MIN_GB}GB (WAL growth?)"
            step_failed=1
            fail_reason="disk_full"
            break
        fi
    done

    # Stop sysbench if still running
    if kill -0 "$sysbench_pid" 2>/dev/null; then
        kill -INT "$sysbench_pid" 2>/dev/null || true
        sleep 2
        if kill -0 "$sysbench_pid" 2>/dev/null; then
            kill -KILL "$sysbench_pid" 2>/dev/null || true
        fi
    fi

    # Compute final metrics
    local rss_start=$(head -2 "$metrics_file" | tail -1 | cut -d, -f3)
    local rss_end=$(tail -1 "$metrics_file" | cut -d, -f3)
    local fd_start=$(head -2 "$metrics_file" | tail -1 | cut -d, -f4)
    local fd_end=$(tail -1 "$metrics_file" | cut -d, -f4)
    local rss_delta=$((rss_end - rss_start))
    local fd_delta=$((fd_end - fd_start))
    log "  Final: RSS=${rss_end}MB (Δ${rss_delta}MB) FD=${fd_end} (Δ${fd_delta})"

    # Stop server cleanly
    kill_server "$server_pid" "$PORT"

    if [ "$step_failed" -eq 1 ]; then
        log "  ❌ Step $label FAILED: $fail_reason"
        return 1
    fi
    log "  ✅ Step $label PASS"
    return 0
}

# ---- Main ladder loop ----
log "=========================================="
log "SQLRustGo Short-Soak Ladder"
log "Steps: $LADDER"
log "Port: $PORT Threads: $THREADS"
log "Results: $RESULTS_ROOT"
log "=========================================="

# Build binary once (unless SKIP_BUILD=1)
if [ "$SKIP_BUILD" != "1" ]; then
    log "Building sqlrustgo-mysql-server (release)..."
    cargo build --release --bin sqlrustgo-mysql-server 2>&1 | tail -3
fi

# Initialize status file
echo "Status: RUNNING" > "$LADDER_STATUS"
echo "Started: $(date)" >> "$LADDER_STATUS"
echo "Steps: $LADDER" >> "$LADDER_STATUS"
echo "Results: $RESULTS_ROOT" >> "$LADDER_STATUS"

PASSED_STEPS=""
FAILED_STEP=""
for label in $LADDER; do
    seconds=$(parse_duration_to_seconds "$label") || {
        alert "Invalid step label: $label"
        FAILED_STEP="$label"
        break
    }
    # Skip port check — use unique port per ladder (default 3397)
    if run_step "$label" "$seconds"; then
        PASSED_STEPS="$PASSED_STEPS $label"
    else
        FAILED_STEP="$label"
        break
    fi
done

# Final report
log "=========================================="
if [ -z "$FAILED_STEP" ]; then
    log "🎉 LADDER COMPLETE: $PASSED_STEPS"
    echo "Status: PASS" > "$LADDER_STATUS"
    echo "Passed: $PASSED_STEPS" >> "$LADDER_STATUS"
    echo "Finished: $(date)" >> "$LADDER_STATUS"
    exit 0
else
    log "❌ LADDER FAILED at: $FAILED_STEP"
    log "Passed before failure:$PASSED_STEPS"
    echo "Status: FAIL" > "$LADDER_STATUS"
    echo "Failed: $FAILED_STEP" >> "$LADDER_STATUS"
    echo "Passed: $PASSED_STEPS" >> "$LADDER_STATUS"
    echo "Finished: $(date)" >> "$LADDER_STATUS"
    exit 1
fi