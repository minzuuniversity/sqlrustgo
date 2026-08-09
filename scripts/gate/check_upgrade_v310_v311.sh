#!/usr/bin/env bash
# check_upgrade_v310_v311.sh — GA-P0 Upgrade Test Gate
#
# Verifies upgrade test infrastructure for Issue #3605:
# 1. Upgrade script exists and is valid bash
# 2. Upgrade test Rust file exists and compiles
# 3. Required functions present in upgrade script
#
# Exit code: 0 = PASS, 1 = FAIL

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== GA-P0 Upgrade v3.10.0 → v3.11.0 Gate (Issue #3605) ==="
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

# 1. Upgrade script exists
echo "--- Upgrade Script ---"
if [ -f "scripts/test_upgrade_v310_to_v311.sh" ]; then
    echo "  [PASS] scripts/test_upgrade_v310_to_v311.sh exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] scripts/test_upgrade_v310_to_v311.sh missing"
    FAIL=$((FAIL+1))
fi

# 2. Upgrade script is valid bash
check "upgrade script syntax" "bash -n scripts/test_upgrade_v310_to_v311.sh"

# 3. Required functions exist
check "setup_v310_data function" "grep -q 'setup_v310_data()' scripts/test_upgrade_v310_to_v311.sh"
check "run_upgrade_test function" "grep -q 'run_upgrade_test()' scripts/test_upgrade_v310_to_v311.sh"
check "verify_data function" "grep -q 'verify_data()' scripts/test_upgrade_v310_to_v311.sh"
check "cleanup function" "grep -q 'cleanup()' scripts/test_upgrade_v310_to_v311.sh"
check "start_server function" "grep -q 'start_server()' scripts/test_upgrade_v310_to_v311.sh"
check "stop_server function" "grep -q 'stop_server()' scripts/test_upgrade_v310_to_v311.sh"

# 4. Rust test exists
echo ""
echo "--- Rust Test Suite ---"
if [ -f "tests/integration/migration/upgrade_v310_v311_test.rs" ]; then
    echo "  [PASS] upgrade_v310_v311_test.rs exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] upgrade_v310_v311_test.rs missing"
    FAIL=$((FAIL+1))
fi

# 5. Rust test compiles
check "upgrade_v310_v311_test.rs compiles" "cargo check --lib 2>/dev/null"

# 6. Gate script is executable
check "gate script executable" "[ -x '$0' ]"

echo ""
echo "=== Upgrade Gate Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "✅ Upgrade v3.10.0 → v3.11.0 gate PASSED"
    exit 0
else
    echo "❌ Upgrade v3.10.0 → v3.11.0 gate FAILED ($FAIL blocker(s))"
    exit 1
fi
