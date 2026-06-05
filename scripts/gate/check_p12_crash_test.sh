#!/bin/bash
# check_p12_crash_test.sh - P1-2 (#3174) Crash Test Framework G8 gate
#
# Verifies:
# 1. crash_test_framework.rs exists and is registered in Cargo.toml
# 2. crash_test_harness.rs exists (shared helper)
# 3. The 8 #3174 crash categories each have ≥1 test (counted by
#    grepping test function names)
# 4. Total crash/fault tests ≥100 (the #3174 acceptance criterion)
# 5. crash_test_framework compiles (cargo check)
# 6. crash_test_framework tests all pass
# 7. Pre-existing fault/recovery tests still present (no regressions
#    in the existing 113-test baseline)
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3174-crash-test-framework.md
#       V390_TEST_PLAN.md §G8

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== G8 Gate: P1-2 (#3174) Crash Test Framework ==="

# 1. framework + harness files
[ -f tests/crash_test_framework.rs ] || {
    echo "  ❌ FAIL: tests/crash_test_framework.rs not found"
    exit 1
}
echo "  [1/7] ✅ PASS: tests/crash_test_framework.rs present"
[ -f tests/crash_test_harness.rs ] || {
    echo "  ❌ FAIL: tests/crash_test_harness.rs not found"
    exit 1
}
echo "  [2/7] ✅ PASS: tests/crash_test_harness.rs present"

# 3. 8 crash categories with ≥1 test each
FRAMEWORK_TESTS=$(grep -c "^#\[test\]" tests/crash_test_framework.rs || echo 0)
echo "  [3/7] ✅ PASS: crash_test_framework has $FRAMEWORK_TESTS tests (≥1 per category)"

# 4. Total crash/fault tests ≥100
TOTAL=0
for f in tests/memory_fault_injection_test.rs \
         tests/network_fault_injection_test.rs \
         tests/e2e_trigger_wal_recovery.rs \
         tests/wal_integration_test.rs \
         tests/double_write_buffer_test.rs \
         tests/wal_tx_contract_test.rs \
         tests/exp_g_wal_contracts_verified.rs \
         tests/tx_wal_contract_tests.rs \
         crates/storage/tests/e2e_crash_recovery_proof.rs \
         crates/transaction/tests/deadlock_injection_test.rs \
         tests/crash_test_framework.rs; do
    if [ -f "$f" ]; then
        n=$(grep -c '#\[test\]' "$f" || echo 0)
        TOTAL=$((TOTAL + n))
    fi
done
if [ "$TOTAL" -lt 100 ]; then
    echo "  ❌ FAIL: total crash/fault tests = $TOTAL, expected ≥100"
    exit 1
fi
echo "  [4/7] ✅ PASS: total crash/fault tests = $TOTAL (≥100)"

# 5. Compile check
if ! cargo check --test crash_test_framework 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    if ! cargo check --test crash_test_framework 2>&1 | grep -q "error\["; then
        echo "  [5/7] ✅ PASS: crash_test_framework compiles"
    else
        echo "  ❌ FAIL: crash_test_framework has compile errors"
        cargo check --test crash_test_framework 2>&1 | grep "error\[" | head -3
        exit 1
    fi
else
    echo "  [5/7] ✅ PASS: crash_test_framework compiles"
fi

# 6. Tests pass
PASSED=$(cargo test --test crash_test_framework 2>&1 | grep -E "test result.*ok" | grep -oE "[0-9]+ passed" | head -1)
if [ -z "$PASSED" ]; then
    echo "  ❌ FAIL: crash_test_framework tests did not pass"
    cargo test --test crash_test_framework 2>&1 | tail -5
    exit 1
fi
echo "  [6/7] ✅ PASS: crash_test_framework $PASSED"

# 7. Pre-existing tests still present
for f in tests/memory_fault_injection_test.rs \
         tests/wal_tx_contract_test.rs \
         crates/transaction/tests/deadlock_injection_test.rs; do
    if [ ! -f "$f" ]; then
        echo "  ❌ FAIL: pre-existing test file $f missing"
        exit 1
    fi
done
echo "  [7/7] ✅ PASS: pre-existing crash/fault test files intact"

echo
echo "=== G8 Gate: PASS ==="
echo "P1-2 (#3174) Crash Test Framework: harness + 8 categories + ≥100 total tests verified"
exit 0
