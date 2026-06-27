#!/bin/bash
# run_soak_staged.sh - 3-phase SOAK orchestrator (6h -> 24h -> 72h).
#                     Reuses run_wired_soak.sh for each phase.
#                     Verifies exit criteria between phases.
#
# Usage:
#   bash scripts/stability/run_soak_staged.sh
#
# Environment:
#   STAGES        comma-separated HOURS list (default "6,24,72")
#   RESULTS_DIR   base output dir (default test_results/staged_soak_<ts>)
#   SKIP_PHASE1   if set, skip 6h phase (run only 24h+72h)
#   SKIP_PHASE2   if set, skip 24h phase (run only 6h+72h)
#   SKIP_PHASE3   if set, skip 72h phase (run only 6h+24h)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

STAGES="${STAGES:-6,24,72}"
RESULTS_DIR="${RESULTS_DIR:-test_results/staged_soak_$(date +%Y%m%d_%H%M%S)}"
mkdir -p "$RESULTS_DIR"

echo "=== 72h SOAK Stage-Gate Runner ==="
echo "Stages: $STAGES"
echo "Results dir: $RESULTS_DIR"
echo ""

run_phase() {
    local hours=$1
    local phase_name=$2
    local phase_dir="$RESULTS_DIR/$phase_name"

    echo ""
    echo "=== Phase: $phase_name (${hours}h) ==="
    echo "Output dir: $phase_dir"
    mkdir -p "$phase_dir"

    HOURS="$hours" \
    RESULTS_DIR="$phase_dir" \
    TPCH_ROTATE_INTERVAL=600 \
    THREADS=16 \
    SERVER_THREADS=16 \
    TABLE_SIZE=10000 \
    FIXTURE=tpch-sf001 \
    SKIP_SYSBENCH="${SKIP_SYSBENCH:-0}" \
    bash scripts/stability/run_wired_soak.sh > "$phase_dir/soak.log" 2>&1 &
    local soak_pid=$!

    echo "  SOAK PID: $soak_pid"
    echo "  Waiting for completion (${hours}h + buffer)..."

    local max_secs=$(( hours * 3600 + 600 ))
    local elapsed=0
    while kill -0 "$soak_pid" 2>/dev/null && [ $elapsed -lt $max_secs ]; do
        sleep 60
        elapsed=$(( elapsed + 60 ))

        if [ $(( elapsed % 1800 )) -eq 0 ]; then
            echo "  [${elapsed}s] SOAK still running. Checking server health..."
            local server_pid=$(pgrep -f "sqlrustgo-mysql-server serve" | head -1 || true)
            if [ -n "$server_pid" ]; then
                local rss=$(ps -o rss= -p "$server_pid" 2>/dev/null || echo 0)
                local fd=$(ls /proc/$server_pid/fd 2>/dev/null | wc -l || echo 0)
                echo "    RSS: ${rss} KB, FD: $fd"
            else
                echo "    WARNING: server process not found"
            fi
        fi
    done

    if kill -0 "$soak_pid" 2>/dev/null; then
        echo "  ERROR: SOAK did not complete in ${max_secs}s. Killing."
        kill -9 "$soak_pid" 2>/dev/null || true
        return 1
    fi

    wait "$soak_pid" || {
        echo "  ERROR: SOAK exited with non-zero status."
        return 1
    }

    echo "  Phase $phase_name completed."
    return 0
}

check_phase_health() {
    local phase_name=$1
    local phase_dir="$RESULTS_DIR/$phase_name"
    local metrics="$phase_dir/metrics.csv"

    echo ""
    echo "=== Phase Health Check: $phase_name ==="

    if [ ! -f "$metrics" ]; then
        echo "  FAIL: $metrics not found"
        return 1
    fi

    local last_rss=$(tail -10 "$metrics" | awk -F, '$1=="rss_mb" {print $2}' | tail -1)
    local last_wal=$(tail -10 "$metrics" | awk -F, '$1=="wal_mb" {print $2}' | tail -1)
    local last_fd=$(tail -10 "$metrics" | awk -F, '$1=="fd_count" {print $2}' | tail -1)

    echo "  Last RSS: ${last_rss:-UNKNOWN} MB"
    echo "  Last WAL: ${last_wal:-UNKNOWN} MB"
    echo "  Last FD:  ${last_fd:-UNKNOWN}"

    local rss_limit=1024
    local wal_limit=1024
    local fd_limit=200

    case "$phase_name" in
        phase1_6h)  rss_limit=500; wal_limit=1024; fd_limit=100 ;;
        phase2_24h) rss_limit=1024; wal_limit=1024; fd_limit=150 ;;
        phase3_72h) rss_limit=1024; wal_limit=100;  fd_limit=200 ;;
    esac

    local failed=0
    if [ "${last_rss:-0}" -gt "$rss_limit" ] 2>/dev/null; then
        echo "  FAIL: RSS ${last_rss} MB > limit ${rss_limit} MB"
        failed=1
    fi
    if [ "${last_wal:-0}" -gt "$wal_limit" ] 2>/dev/null; then
        echo "  FAIL: WAL ${last_wal} MB > limit ${wal_limit} MB"
        failed=1
    fi
    if [ "${last_fd:-0}" -gt "$fd_limit" ] 2>/dev/null; then
        echo "  FAIL: FD ${last_fd} > limit ${fd_limit}"
        failed=1
    fi

    if [ "$failed" -eq 0 ]; then
        echo "  PASS: All exit criteria met."
        return 0
    else
        echo "  FAIL: One or more exit criteria not met."
        return 1
    fi
}

SKIP_PHASES=""
[ -n "${SKIP_PHASE1:-}" ] && SKIP_PHASES="$SKIP_PHASES phase1_6h"
[ -n "${SKIP_PHASE2:-}" ] && SKIP_PHASES="$SKIP_PHASES phase2_24h"
[ -n "${SKIP_PHASE3:-}" ] && SKIP_PHASES="$SKIP_PHASES phase3_72h"

IFS=',' read -ra STAGE_LIST <<< "$STAGES"
PHASE_NUM=0

for hours in "${STAGE_LIST[@]}"; do
    PHASE_NUM=$(( PHASE_NUM + 1 ))
    case "$hours" in
        6)  phase_name="phase1_6h" ;;
        24) phase_name="phase2_24h" ;;
        72) phase_name="phase3_72h" ;;
        *)  phase_name="phase${PHASE_NUM}_${hours}h" ;;
    esac

    if echo "$SKIP_PHASES" | grep -qw "$phase_name"; then
        echo "SKIP: $phase_name (SKIP_PHASE${PHASE_NUM} set)"
        continue
    fi

    if ! run_phase "$hours" "$phase_name"; then
        echo ""
        echo "FATAL: Phase $phase_name failed. Aborting."
        echo "Partial results in: $RESULTS_DIR"
        exit 1
    fi

    if [ "$hours" != "${STAGE_LIST[-1]}" ]; then
        if ! check_phase_health "$phase_name"; then
            echo ""
            echo "FATAL: Phase $phase_name health check failed. NOT proceeding to next phase."
            echo "Investigate: $RESULTS_DIR/$phase_name"
            exit 2
        fi
    fi
done

echo ""
echo "=== All phases completed successfully ==="
echo "Results: $RESULTS_DIR"
echo "Run aggregate_soak_report.sh next."