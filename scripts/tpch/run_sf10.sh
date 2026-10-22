#!/usr/bin/env bash
# run_sf10.sh — Run TPC-H SF=10 benchmark
#
# Usage:
#   bash scripts/tpch/run_sf10.sh [DATA_DIR] [TIMEOUT] [all|summary|Q#]
#
# Runs all 22 TPC-H queries against SF=10 data and records
# execution time and memory usage.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# #5516: the binary lookup goes through the repo's single shell-side
# resolver (CARGO_TARGET_DIR-aware) instead of a hardcoded
# `./target/release/...` path.
source "$REPO_ROOT/scripts/common/resolve_bin.sh"

# Configuration
DATA_DIR="${1:-/tmp/tpch-sf10}"
TIMEOUT="${2:-600}"  # 10 minutes per query
# The mode is the **third** argument. `main` is called without positional
# parameters, so `case "${1:-all}"` inside `main` used to look at an empty
# `$1` and always fall through to the `all` branch — the documented
# `Q#` mode was unreachable dead code.
MODE="${3:-all}"
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

    # #5516: the runner used to be `tpch_runner`. **That binary has never
    # existed in this repository** — no Rust source, no Cargo target, in the
    # entire git history (the script was introduced by "GA-P1 #3607" already
    # referring to it). The lookup therefore always failed, and the script
    # wrote a stub "SKIPPED" report and returned 0 — so every SF=10 run
    # reported "22/22 PASS" while **not one query had ever been executed**.
    #
    # The real runner is the cargo example `tpch_run_query`
    # (`crates/bench/examples/tpch_run_query.rs`): it takes
    # `--query <name>` (reading `queries/<name>.sql`), `--data-dir` and
    # `--timeout`, and prints one JSON object.
    #
    # A missing runner is a **hard failure**: a benchmark that did not run
    # must not be able to report a pass. The runner is resolved once, in
    # `main`, before any query is attempted.
    local runner="${TPCH_RUNNER:-}"
    if [ -z "$runner" ]; then
        log_error "internal: the tpch runner was not resolved before the query"
        return 1
    fi

    # Run query with timeout.
    #
    # `tpch_run_query` exits 0 even when the query itself failed — the
    # error is inside the JSON it prints. Judging by the exit code alone
    # would therefore turn 22 failed queries into "22/22 PASSED", which is
    # exactly the claim the governance audit had to retract once already.
    # The verdict is read from the JSON's `error` field.
    local raw_out=""
    local exit_code=0
    raw_out="$(timeout "$TIMEOUT" "$runner" --query "$q" --data-dir "$DATA_DIR" \
        --timeout "$TIMEOUT" 2>&1)" || exit_code=$?

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    printf '%s\n' "$raw_out" > "$log_file"
    echo "Duration: ${duration}s" >> "$log_file"

    if [ "$exit_code" -eq 124 ]; then
        status="TIMEOUT"
        log_error "Q$q: TIMEOUT after ${TIMEOUT}s"
    elif [ "$exit_code" -ne 0 ]; then
        status="FAIL"
        log_error "Q$q: FAIL (exit $exit_code, ${duration}s)"
    elif ! printf '%s' "$raw_out" | grep -q '"error"[[:space:]]*:[[:space:]]*null'; then
        # The runner exited 0 but reported an error, or printed something
        # that is not the expected JSON at all. Either way it did not pass.
        status="FAIL"
        log_error "Q$q: FAIL (the runner reported an error)"
    elif printf '%s' "$raw_out" | grep -q '"timed_out"[[:space:]]*:[[:space:]]*true'; then
        # The runner finished on its own but reports that it blew through its
        # own deadline. The exit code says nothing about that, so the JSON has
        # to be consulted as well — otherwise a query that ran for twice the
        # allowed time would be counted as a pass.
        status="TIMEOUT"
        log_error "Q$q: TIMEOUT (the runner reported timed_out=true, ${duration}s)"
    else
        status="PASS"
        log_query "$q: PASS (${duration}s)"
    fi

    # The **status line goes last**: `run_all` and `show_summary` read the
    # verdict with `tail -1`, so anything appended after it would be read as
    # the verdict instead.
    if [ "$status" != "PASS" ]; then
        echo "Exit code: $exit_code" >> "$log_file"
    fi
    echo "$status" >> "$log_file"
    if [ "$status" != "PASS" ]; then
        return 1
    fi
    return 0
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
        # A failed query must not abort the sweep — the point of the sweep is
        # the tally. It is not swallowed either: the failure is reported and
        # counted below.
        run_query "$q" || log_warn "Q$q did not pass (see $RESULTS_DIR/q${q}.log)"
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

    # #5516: the runner is resolved **once**, up front. A benchmark whose
    # runner is missing must fail immediately and loudly — the old code
    # looked for `tpch_runner` (which has never existed) inside the
    # per-query loop, wrote a "SKIPPED" stub and returned 0, so the whole
    # SF=10 run reported success without executing a single query.
    if [ -z "${TPCH_RUNNER:-}" ]; then
        TPCH_RUNNER="$(resolve_cargo_bin tpch_run_query TPCH_RUNNER run_sf10)" || {
            log_error "the tpch_run_query binary was not found."
            log_error "Build it: cargo build -p sqlrustgo-bench --example tpch_run_query"
            log_error "or point TPCH_RUNNER at an existing binary."
            exit 1
        }
    fi
    log_info "Runner: $TPCH_RUNNER"

    case "$MODE" in
        all)
            # The verdict has to survive `show_summary`: without capturing it
            # here the script would exit with the status of the *last* command
            # in the branch (`show_summary`, i.e. 0) and a run with failed
            # queries would still report success.
            local overall=0
            run_all || overall=$?
            show_summary
            return "$overall"
            ;;
        summary)
            show_summary
            return 0
            ;;
        q[0-9]*|q[0-9][0-9]*)
            local qnum="${1#q}"
            local overall=0
            run_query "$qnum" || overall=$?
            cat "$RESULTS_DIR/q${qnum}.log"
            return "$overall"
            ;;
        *)
            echo "Usage: $0 [DATA_DIR] [TIMEOUT] [all|summary|Q#]"
            return 1
            ;;
    esac
}

main
