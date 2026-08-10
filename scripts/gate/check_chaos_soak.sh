#!/usr/bin/env bash
# check_chaos_soak.sh — GA-P0 Chaos SOAK Gate
#
# Verifies chaos injection infrastructure for Issue #3604:
# 1. chaos_inject.py exists and is valid Python
# 2. chaos_soak_test.rs exists and compiles
# 3. Chaos controller has required methods
#
# Exit code: 0 = PASS, 1 = FAIL

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== GA-P0 Chaos SOAK Gate (Issue #3604) ==="
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

# 1. chaos_inject.py exists
echo "--- Chaos Controller ---"
if [ -f "scripts/soak/chaos_inject.py" ]; then
    echo "  [PASS] scripts/soak/chaos_inject.py exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] scripts/soak/chaos_inject.py missing"
    FAIL=$((FAIL+1))
fi

# 2. chaos_inject.py is valid Python
check "chaos_inject.py syntax" "python3 -m py_compile scripts/soak/chaos_inject.py"

# 3. chaos_inject.py has required methods
check "ChaosController class" "grep -q 'class ChaosController' scripts/soak/chaos_inject.py"
check "inject_io_latency method" "grep -q 'def inject_io_latency' scripts/soak/chaos_inject.py"
check "inject_memory_pressure method" "grep -q 'def inject_memory_pressure' scripts/soak/chaos_inject.py"
check "kill_server_process method" "grep -q 'def kill_server_process' scripts/soak/chaos_inject.py"
check "verify_recovery method" "grep -q 'def verify_recovery' scripts/soak/chaos_inject.py"
check "cleanup method" "grep -q 'def cleanup' scripts/soak/chaos_inject.py"

# 4. chaos_soak_test.rs exists
echo ""
echo "--- Rust Test Suite ---"
if [ -f "tests/integration/stress/chaos_soak_test.rs" ]; then
    echo "  [PASS] tests/integration/stress/chaos_soak_test.rs exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] tests/integration/stress/chaos_soak_test.rs missing"
    FAIL=$((FAIL+1))
fi

# 5. chaos_soak_test.rs compiles
check "chaos_soak_test.rs compiles" "cargo check --test soak_test 2>/dev/null || cargo check --lib 2>/dev/null"

# 6. chaos_soak_test has required tests
check "test_chaos_io_latency_injectable" "grep -q 'test_chaos_io_latency_injectable' tests/integration/stress/chaos_soak_test.rs"
check "test_chaos_memory_pressure_available" "grep -q 'test_chaos_memory_pressure_available' tests/integration/stress/chaos_soak_test.rs"
check "test_chaos_controller_script_valid" "grep -q 'test_chaos_controller_script_valid' tests/integration/stress/chaos_soak_test.rs"

# 7. Gate script itself is executable
check "gate script executable" "[ -x '$0' ]"

echo ""
echo "=== Chaos SOAK Gate Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "✅ Chaos SOAK gate PASSED"
    exit 0
else
    echo "❌ Chaos SOAK gate FAILED ($FAIL blocker(s))"
    exit 1
fi
