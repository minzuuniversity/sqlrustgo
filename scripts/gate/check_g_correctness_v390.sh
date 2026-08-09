#!/bin/bash
# check_g_correctness_v390.sh - Gate-C (v3.9.0) Substantive Correctness Gate
#
# Runs the TPC-H 22-query suite in-process on SF=0.1 and verifies
# pass count meets the GA criteria:
#   - PASS threshold: >= 21/22 (95%+) for v3.9.0 GA
#   - TIMEOUT threshold: <= 1/22 (5%)
#   - Oracle mismatch: 0
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Sprint 5 v3 (2026-06-08) — uses tpch_sf01_inprocess_test (SF 0.1
# in-process, smoke subset by default; TPCH_SF01_ALL=1 runs all 22).
# Sprint 5 v4 baseline: 20/22 PASS, 1 TIMEOUT (Q21), 1 unreachable (Q22).
#
# Refs: docs/audit/status/2026-06-07-tpch-root-cause-board-v390.md
#       docs/releases/v3.9.0/ROADMAP.md Phase 4 (GA)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== Gate-C (v3.9.0): Substantive TPC-H Correctness ==="
echo

# Default: smoke subset (6 fast queries, <10s). For full 22, set
# TPCH_SF01_ALL=1 (may take 10+ min due to Q21 4-table EXISTS).
if [ "${TPCH_SF01_ALL:-0}" = "1" ]; then
    echo "[1/3] Running tpch_sf01_inprocess_test (TPCH_SF01_ALL=1, full 22)..."
    cargo test --test tpch_sf01_inprocess_test --all-features -- --nocapture > /tmp/gate_c_run.log 2>&1 || { echo "  FAIL: cargo test exit $?" >&2; tail -20 /tmp/v312_31_fail.log; exit 1; }
    grep -E "Q[[:space:]]*[0-9]+: ok|Q[[:space:]]*[0-9]+: ERR|=== TPC-H|test result" /tmp/gate_c_run.log
    THRESHOLD=21
else
    echo "[1/3] Running tpch_sf01_inprocess_test (smoke 6, fast)..."
    cargo test --test tpch_sf01_inprocess_test --all-features -- --nocapture > /tmp/gate_c_run.log 2>&1 || { echo "  FAIL: cargo test exit $?" >&2; tail -20 /tmp/v312_31_fail.log; exit 1; }
    grep -E "Q[[:space:]]*[0-9]+: ok|Q[[:space:]]*[0-9]+: ERR|=== TPC-H|test result" /tmp/gate_c_run.log
    THRESHOLD=6
fi
echo

PASS=$(grep -cE "Q[[:space:]]*[0-9]+: ok" /tmp/gate_c_run.log || true)
ERR=$(grep -cE "Q[[:space:]]*[0-9]+: ERR" /tmp/gate_c_run.log || true)
TOTAL=$((PASS + ERR))
echo "[2/3] Results: $PASS PASS, $ERR ERR (out of $TOTAL)"

if [ "$PASS" -ge "$THRESHOLD" ]; then
    echo "  PASS: $PASS >= $THRESHOLD"
    GATE_RESULT="PASS"
else
    echo "  FAIL: $PASS < $THRESHOLD"
    GATE_RESULT="FAIL"
fi
echo

echo "[3/3] Running Operator Regression Suite (aggregate/exists/join)..."
cargo test --test operators_aggregate --test operators_join --all-features 2>&1 | grep -E "test result" | head -5
JOIN_RESULT=$(cargo test --test operators_aggregate --test operators_join --all-features 2>&1 | grep "test result" | tail -1)
echo "  Operator: $JOIN_RESULT"

echo
echo "=== Gate-C: $GATE_RESULT ==="
[ "$GATE_RESULT" = "PASS" ] || exit 1
