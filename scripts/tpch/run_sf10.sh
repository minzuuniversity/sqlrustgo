#!/usr/bin/env bash
# run_sf10.sh — Run TPC-H SF=10 benchmark
#
# Usage:
#   bash scripts/tpch/run_sf10.sh [DATA_DIR] [TIMEOUT]
#
# Runs all 22 TPC-H queries against SF=10 data and records
# execution time and memory usage.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Configuration
DATA_DIR="${1:-/tmp/tpch-sf10}"
TIMEOUT="${2:-600}"  # 10 minutes per query
RESULTS_DIR="${RESULTS_DIR:-/tmp/tpch-sf10-results}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_query() { echo -e "${GREEN}[Q$1]${NC} $*"; }

# Create results directory
mkdir -p "$RESULTS_DIR"

# Check data exists
check_data() {
    if [ ! -d "$DATA_DIR" ]; then
        log_error "Data directory not found: $DATA_DIR"
        log_info "Run: bash scripts/tpch/setup_sf10.sh first"
        return 1
    fi

    local tables=(customer orders lineitem part supplier partsupp nation region)
    for table in "${tables[@]}"; do
        if [ ! -f "$DATA_DIR/${table}.tbl" ]; then
            log_warn "Missing ${table}.tbl"
        fi
    done

    log_info "Data directory: $DATA_DIR"
}

# Run a single query with timeout and memory monitoring
run_query() {
    local q="$1"
    local start_time=$(date +%s)
    local start_rss=0
    local peak_rss=0
    local log_file="$RESULTS_DIR/q${q}.log"
    local status="PASS"

    log_query "$q: Starting..."

    # Check if tpch_runner exists
    local runner=""
    if [ -x "$REPO_ROOT/target/release/tpch_runner" ]; then
        runner="$REPO_ROOT/target/release/tpch_runner"
    elif [ -x "$REPO_ROOT/target/debug/tpch_runner" ]; then
        runner="$REPO_ROOT/target/debug/tpch_runner"
    fi

    if [ -z "$runner" ]; then
        log_warn "tpch_runner not found. Recording stub result."
        echo "SKIPPED: tpch_runner not built" > "$log_file"
        return 0
    fi

    # Run query with timeout
    if timeout "$TIMEOUT" "$runner" --query "$q" --sf 10 --data-dir "$DATA_DIR" > "$log_file" 2>&1; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))

        log_query "$q: PASS (${duration}s)"
        echo "PASS" >> "$log_file"
        echo "Duration: ${duration}s" >> "$log_file"
    else
        local exit_code=$?
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))

        if [ $exit_code -eq 124 ]; then
            status="TIMEOUT"
            log_error "Q$q: TIMEOUT after ${TIMEOUT}s"
        else
            status="FAIL"
            log_error "Q$q: FAIL (exit $exit_code, ${duration}s)"
        fi

        echo "$status" >> "$log_file"
        echo "Duration: ${duration}s" >> "$log_file"
        echo "Exit code: $exit_code" >> "$log_file"
    fi
}

# Run all 22 queries
run_all() {
    log_info "=== TPC-H SF=10 Benchmark ==="
    log_info "Data: $DATA_DIR"
    log_info "Timeout: ${TIMEOUT}s per query"
    log_info "Results: $RESULTS_DIR"
    echo ""

    local start_total=$(date +%s)
    local pass=0
    local fail=0
    local timeout_count=0

    for q in $(seq 1 22); do
        run_query "$q"
        local result=$(tail -1 "$RESULTS_DIR/q${q}.log" 2>/dev/null || echo "UNKNOWN")

        case "$result" in
            PASS) pass=$((pass + 1)) ;;
            TIMEOUT) timeout_count=$((timeout_count + 1)) ;;
            *) fail=$((fail + 1)) ;;
        esac
    done

    local end_total=$(date +%s)
    local total_duration=$((end_total - start_total))

    echo ""
    log_info "=== Results ==="
    log_info "Total time: ${total_duration}s"
    log_info "PASS: $pass/22"
    log_info "TIMEOUT: $timeout_count/22"
    log_info "FAIL: $fail/22"

    if [ $fail -eq 0 ] && [ $timeout_count -eq 0 ]; then
        log_info "=== ALL 22 QUERIES PASSED ==="
        return 0
    else
        log_error "=== SOME QUERIES FAILED ==="
        return 1
    fi
}

# Show summary
show_summary() {
    log_info "=== Query Summary ==="
    printf "%-4s %-10s %s\n" "Q#" "Status" "Duration"
    printf "%-4s %-10s %s\n" "---" "------" "--------"

    for q in $(seq 1 22); do
        local log_file="$RESULTS_DIR/q${q}.log"
        if [ -f "$log_file" ]; then
            local status=$(tail -1 "$log_file")
            local duration=$(grep "^Duration:" "$log_file" 2>/dev/null | cut -d: -f2 | tr -d ' ')
            printf "%-4s %-10s %s\n" "Q$q" "$status" "${duration:-N/A}"
        else
            printf "%-4s %-10s %s\n" "Q$q" "NOT RUN" "-"
        fi
    done
}

# Main
main() {
    check_data || exit 1

    case "${1:-all}" in
        all)
            run_all
            show_summary
            ;;
        summary)
            show_summary
            ;;
        q[0-9]*|q[0-9][0-9]*)
            local qnum="${1#q}"
            run_query "$qnum"
            cat "$RESULTS_DIR/q${qnum}.log"
            ;;
        *)
            echo "Usage: $0 [all|summary|Q#]"
            exit 1
            ;;
    esac
}

main
