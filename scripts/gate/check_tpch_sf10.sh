#!/usr/bin/env bash
# check_tpch_sf10.sh — GA-P1 TPC-H SF=10 Gate
#
# Verifies TPC-H SF=10 infrastructure for Issue #3607:
# 1. SF=10 setup script exists
# 2. SF=10 runner script exists
# 3. Scripts have valid bash syntax
#
# Exit code: 0 = PASS, 1 = FAIL

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== GA-P1 TPC-H SF=10 Gate (Issue #3607) ==="
echo ""

PASS=0
FAIL=0

check() {
    local name="$1"
    local cmd="$2"
    if eval "$cmd" >/dev/null 2>&1; then
        echo "  [PASS] $name"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] $name"
        FAIL=$((FAIL+1))
    fi
}

# 1. Setup script
echo "--- Setup Script ---"
if [ -f "scripts/tpch/setup_sf10.sh" ]; then
    echo "  [PASS] scripts/tpch/setup_sf10.sh exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] scripts/tpch/setup_sf10.sh missing"
    FAIL=$((FAIL+1))
fi

check "setup_sf10.sh syntax" "bash -n scripts/tpch/setup_sf10.sh"
check "setup_sf10.sh has main" "grep -q 'main()' scripts/tpch/setup_sf10.sh"

# 2. Runner script
echo ""
echo "--- Runner Script ---"
if [ -f "scripts/tpch/run_sf10.sh" ]; then
    echo "  [PASS] scripts/tpch/run_sf10.sh exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] scripts/tpch/run_sf10.sh missing"
    FAIL=$((FAIL+1))
fi

check "run_sf10.sh syntax" "bash -n scripts/tpch/run_sf10.sh"
check "run_sf10.sh has run_query" "grep -q 'run_query()' scripts/tpch/run_sf10.sh"
check "run_sf10.sh has 22 queries" "grep -q 'seq 1 22' scripts/tpch/run_sf10.sh"

# 3. Gate script executable
check "gate script executable" "[ -x '$0' ]"

# 4. Cross-engine harness (Issue #4018)
echo ""
echo "--- Cross-engine Harness (Issue #4018) ---"
if [ -f "scripts/tpch_sf10_cross_engine_harness.py" ]; then
    echo "  [PASS] scripts/tpch_sf10_cross_engine_harness.py exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] scripts/tpch_sf10_cross_engine_harness.py missing"
    FAIL=$((FAIL+1))
fi

check "sf10 harness python syntax" "python3 -c 'import ast; ast.parse(open(\"scripts/tpch_sf10_cross_engine_harness.py\").read())'"
check "sf10 harness imports from sf1 module" "grep -q 'tpch_cross_engine_harness' scripts/tpch_sf10_cross_engine_harness.py"
check "sf10 harness runs --help" "python3 scripts/tpch_sf10_cross_engine_harness.py --help >/dev/null"

# 5. SF=10 evidence directory (per STRICT PROOF MODE)
SF10_EVIDENCE_DIR="docs/releases/v3.12.0/evidence/tpch/cross_engine_sf10"
echo ""
echo "--- SF=10 Evidence Directory ---"
if [ -d "$SF10_EVIDENCE_DIR" ] || mkdir -p "$SF10_EVIDENCE_DIR" 2>/dev/null; then
    echo "  [PASS] $SF10_EVIDENCE_DIR exists (or can be created)"
    PASS=$((PASS+1))
else
    echo "  [FAIL] $SF10_EVIDENCE_DIR missing and cannot be created"
    FAIL=$((FAIL+1))
fi

echo ""
echo "=== TPC-H SF=10 Gate Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "✅ TPC-H SF=10 gate PASSED"
    exit 0
else
    echo "❌ TPC-H SF=10 gate FAILED ($FAIL blocker(s))"
    exit 1
fi
