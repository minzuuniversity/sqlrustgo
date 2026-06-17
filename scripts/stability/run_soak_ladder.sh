#!/bin/bash
# run_soak_ladder.sh - Progressive soak test ladder: 1→2→4→8→16→24→48→72 hours
#
# Each step: starts server + sysbench + resource guard
# If step PASS → proceed to next step
# If step FAIL → stop ladder and report
#
# Resource guard thresholds (applied to each step):
#   RSS_HARD_LIMIT_MB   Kill if RSS > 8192 MB
#   RSS_SOFT_MB        Warn if RSS > 4096 MB
#   RSS_GROWTH_RATE    Warn if RSS growth > 200 MB/hr
#   DISK_MIN_GB        Kill if disk < 2 GB
#   FD_HARD_LIMIT      Kill if FD > 8000
#
# Usage:
#   bash scripts/stability/run_soak_ladder.sh
#   PORT=3396 THREADS=8 bash scripts/stability/run_soak_ladder.sh
#
# Cron monitoring (every 5 min):
#   */5 * * * * bash /path/to/scripts/stability/check_ladder_status.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# ---- Config ----
PORT="${PORT:-3396}"
THREADS="${THREADS:-8}"
HOST="${HOST:-127.0.0.1}"
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-$PROJECT_ROOT/target/release/sqlrustgo-mysql-server}"
INTERVAL="${INTERVAL:-30}"
LADDER_LOG="${LADDER_LOG:-test_results/soak_ladder.log}"
LADDER_STATUS="${LADDER_STATUS:-test_results/soak_ladder_STATUS.txt}"

# Resource thresholds
RSS_HARD_LIMIT_MB="${RSS_HARD_LIMIT_MB:-8192}"
RSS_SOFT_MB="${RSS_SOFT_MB:-4096}"
RSS_GROWTH_RATE_MB_PER_HR="${RSS_GROWTH_RATE_MB_PER_HR:-200}"
DISK_MIN_GB="${DISK_MIN_GB:-2}"
FD_HARD_LIMIT="${FD_HARD_LIMIT:-8000}"

# Ladder definition
LADDER=(1 2 4 8 16 24 48 72)

# ---- Helper functions ----
log() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] $*" | tee -a "$LADDER_LOG"; }
alert() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] ALERT: $*" | tee -a "$LADDER_LOG" >&2; }

check_port() {
    if lsof -i ":$1" -sTCP:LISTEN >/dev/null 2>&1; then
        return 0  # in use
    fi
    return 1  # free
}

wait_port_free() {
    local port=$1
    local max_wait=${2:-60}
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

check_port() {
    lsof -i ":$1" -sTCP:LISTEN >/dev/null 2>&1
}

run_single_step() {
    local hours=$1
    local step_log="$RESULTS_DIR/step_${hours}h.log"
    local step_metrics="$RESULTS_DIR/step_${hours}h_metrics.csv"
    local step_report="$RESULTS_DIR/step_${hours}h_REPORT.md"
    
    log "=== LADDER STEP: ${hours}H starting ==="
    echo "$hoursH" > "$LADDER_STATUS"
    echo "RUNNING" >> "$LADDER_STATUS"
    
    RESULTS_DIR="$RESULTS_DIR"     HOURS="$hours"     THREADS="$THREADS"     PORT="$PORT"     HOST="$HOST"     INTERVAL="$INTERVAL"     SQLRUSTGO_BIN="$SQLRUSTGO_BIN"     RSS_HARD_LIMIT_MB="$RSS_HARD_LIMIT_MB"     RSS_SOFT_MB="$RSS_SOFT_MB"     RSS_GROWTH_RATE_MB_PER_HR="$RSS_GROWTH_RATE_MB_PER_HR"     DISK_MIN_GB="$DISK_MIN_GB"     FD_HARD_LIMIT="$FD_HARD_LIMIT"     bash "$SCRIPT_DIR/run_soak_single.sh" > "$step_log" 2>&1
    local exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        log "=== LADDER STEP: ${hours}H PASSED ==="
        echo "PASS" >> "$LADDER_STATUS"
        return 0
    else
        log "=== LADDER STEP: ${hours}H FAILED (exit=$exit_code) ==="
        echo "FAIL" >> "$LADDER_STATUS"
        return $exit_code
    fi
}

# ---- Main ----
# Find free port
BASE_PORT=$PORT
while check_port $BASE_PORT; do
    BASE_PORT=$((BASE_PORT + 1))
done
PORT=$BASE_PORT
log "Using port: $PORT"

# Prepare results dir
RESULTS_DIR="${RESULTS_DIR:-test_results/soak_ladder_$(date +%Y%m%d_%H%M%S)}"
mkdir -p "$RESULTS_DIR"

echo "INIT" > "$LADDER_STATUS"
echo "PENDING" >> "$LADDER_STATUS"

log "=========================================="
log "SQLRustGo Progressive Soak Ladder"
log "=========================================="
log "Port: $PORT  Threads: $THREADS"
log "Binary: $SQLRUSTGO_BIN"
log "Ladder: ${LADDER[@]}"
log "Results: $RESULTS_DIR"
log "=========================================="

log "Ladder START time: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
LADDER_START_TS=$(date +%s)

PASSED=0
FAILED=0
FAILED_STEP=0
STEP_RESULTS=""

for hours in "${LADDER[@]}"; do
    # Check preconditions
    if ! check_port $PORT; then
        wait_port_free $PORT 60 || {
            alert "Port $PORT still in use after 60s wait. Stopping ladder."
            break
        }
    fi
    
    # Check disk space
    DISK_FREE=$(df -BG /tmp 2>/dev/null | awk 'NR==2 {print $4}' | tr -d 'G')
    if [ "$DISK_FREE" -lt $DISK_MIN_GB ]; then
        alert "Disk /tmp only ${DISK_FREE}GB free. Stopping ladder."
        break
    fi
    
    # Check memory
    RSS_CURRENT=$(ps -o rss= -p $(pgrep -f "sqlrustgo-mysql-server.*serve" | head -1) 2>/dev/null | tr -d ' ' || echo 0)
    RSS_CURRENT_MB=$((RSS_CURRENT / 1024))
    if [ "$RSS_CURRENT_MB" -gt $((RSS_SOFT_MB / 2)) ]; then
        log "RSS ${RSS_CURRENT_MB}MB still elevated from previous run. Waiting..."
        sleep 30
    fi
    
    STEP_START_TS=$(date +%s)
    STEP_RESULT="FAIL"
    
    if run_single_step $hours; then
        STEP_RESULT="PASS"
        PASSED=$((PASSED + 1))
    else
        STEP_RESULT="FAIL"
        FAILED=$((FAILED + 1))
        FAILED_STEP=$hours
        alert "Ladder FAILED at step ${hours}H"
        break
    fi
    
    STEP_END_TS=$(date +%s)
    STEP_DURATION=$((STEP_END_TS - STEP_START_TS))
    STEP_RESULTS="${STEP_RESULTS}  ${hours}H: ${STEP_RESULT} (${STEP_DURATION}s)\n"
    
    # Cool down between steps
    if [ "$hours" != "72" ]; then
        log "Cool-down 60s before next step..."
        sleep 60
    fi
done

LADDER_END_TS=$(date +%s)
LADDER_DURATION=$((LADDER_END_TS - LADDER_START_TS))

log "=========================================="
log "LADDER COMPLETE"
log "=========================================="
log "Duration: ${LADDER_DURATION}s (approx $((LADDER_DURATION / 3600))h wall-clock)"
log "Steps: ${#LADDER[@]} total, $PASSED passed, $FAILED failed"
log "Failed step: ${FAILED_STEP:-none}"
log "Ladder END time: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"

# Generate final report
cat > "$RESULTS_DIR/LADDER_REPORT.md" << EOF
# Progressive Soak Ladder Report

**Start**: $(date -u -d "@$LADDER_START_TS" +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "@$LADDER_START_TS")
**End**: $(date -u -d "@$LADDER_END_TS" +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "@$LADDER_END_TS")
**Duration**: ${LADDER_DURATION}s (approx $((LADDER_DURATION / 3600))h wall-clock)

## Resource Thresholds

| Threshold | Value |
|-----------|-------|
| RSS HARD LIMIT | ${RSS_HARD_LIMIT_MB} MB |
| RSS SOFT WARN | ${RSS_SOFT_MB} MB |
| RSS GROWTH RATE | ${RSS_GROWTH_RATE_MB_PER_HR} MB/hr |
| DISK MIN FREE | ${DISK_MIN_GB} GB |
| FD HARD LIMIT | ${FD_HARD_LIMIT} |

## Ladder Results

| Step | Duration | Result | Artifacts |
|------|----------|--------|-----------|
EOF

step_num=1
for hours in "${LADDER[@]}"; do
    local step_report="$RESULTS_DIR/step_${hours}h_REPORT.md"
    local step_result="❌ FAIL"
    local step_evidence=""
    if [ -f "$step_report" ]; then
        if grep -q "PASS" "$step_report"; then
            step_result="✅ PASS"
        fi
        step_evidence="$RESULTS_DIR/step_${hours}h_REPORT.md"
    fi
    echo "| ${step_num} | ${hours}h | $step_result | $step_evidence |" >> "$RESULTS_DIR/LADDER_REPORT.md"
    step_num=$((step_num + 1))
done

cat >> "$RESULTS_DIR/LADDER_REPORT.md" << EOF

## Verdict

$([ $FAILED -eq 0 ] && echo "**✅ FULL LADDER PASS** — All ${#LADDER[@]} steps passed. System is stable." || echo "**❌ LADDER FAILED** at step ${FAILED_STEP}H. See step_${FAILED_STEP}h_REPORT.md for details.")

## Individual Step Reports

$(for hours in "${LADDER[@]}"; do
    if [ -f "$RESULTS_DIR/step_${hours}h_REPORT.md" ]; then
        echo "### ${hours}H Step"
        cat "$RESULTS_DIR/step_${hours}h_REPORT.md"
        echo ""
    fi
done)
EOF

echo "INIT" > "$LADDER_STATUS"
echo "$([ $FAILED -eq 0 ] && echo "ALL_PASS" || echo "FAILED_AT_${FAILED_STEP}H")" >> "$LADDER_STATUS"
echo "Duration: ${LADDER_DURATION}s" >> "$LADDER_STATUS"

log "Report: $RESULTS_DIR/LADDER_REPORT.md"
cat "$RESULTS_DIR/LADDER_REPORT.md"
log "Ladder complete."
