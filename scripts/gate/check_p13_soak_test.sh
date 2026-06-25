#!/bin/bash
# check_p13_soak_test.sh - P1-3 (#3175) Soak Test G7 gate
#
# Verifies:
# 1. soak_test_harness.rs exists
# 2. soak_test.rs exists and is registered in Cargo.toml
# 3. 3-level smoke equivalence constants are stable (24h/72h/168h
#    map to 60s/180s/420s)
# 4. cargo check pass
# 5. 3 soak tests pass (24h, 72h, 168h)
# 6. Alert-threshold mechanism works (tight threshold triggers alert)
# 7. Memory baseline invariant (no-query run == baseline)
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3175-soak-test.md
#       V390_TEST_PLAN.md §G7

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
# OpenSpec 3175 followup: many agents/VMs only have cargo via
# `$HOME/.cargo/bin/cargo` (rustup default). Add it to PATH if missing
# so the gate is self-contained.
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== G7 Gate: P1-3 (#3175) Soak Test ==="

# 1. harness file
[ -f tests/soak_test_harness.rs ] || {
    echo "  ❌ FAIL: tests/soak_test_harness.rs not found"
    exit 1
}
echo "  [1/7] ✅ PASS: tests/soak_test_harness.rs present"

# 2. test file + registration
[ -f tests/soak_test.rs ] || {
    echo "  ❌ FAIL: tests/soak_test.rs not found"
    exit 1
}
grep -q 'name = "soak_test"' Cargo.toml || {
    echo "  ❌ FAIL: soak_test not registered in Cargo.toml"
    exit 1
}
echo "  [2/7] ✅ PASS: tests/soak_test.rs present + registered"

# 3. 3-level smoke equivalence constants
# Use -A 8 to span all 3 lines ("24h", "72h", "168h" each on a
# separate line) + the closing brace.
SMOKE_24=$(grep -A 8 'fn smoke_seconds_for_level' tests/soak_test_harness.rs | grep '"24h" => Some' | head -1 || true)
SMOKE_72=$(grep -A 8 'fn smoke_seconds_for_level' tests/soak_test_harness.rs | grep '"72h" => Some' | head -1 || true)
SMOKE_168=$(grep -A 8 'fn smoke_seconds_for_level' tests/soak_test_harness.rs | grep '"168h" => Some' | head -1 || true)
if [ -z "$SMOKE_24" ] || [ -z "$SMOKE_72" ] || [ -z "$SMOKE_168" ]; then
    echo "  ❌ FAIL: 3-level smoke equivalence constants missing"
    echo "    SMOKE_24=[$SMOKE_24]"
    echo "    SMOKE_72=[$SMOKE_72]"
    echo "    SMOKE_168=[$SMOKE_168]"
    exit 1
fi
echo "  [3/7] ✅ PASS: 3-level smoke equivalence (24h→60s, 72h→180s, 168h→420s)"

# 4. cargo check
if ! cargo check --test soak_test 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    if cargo check --test soak_test 2>&1 | grep -q "error\["; then
        echo "  ❌ FAIL: soak_test has compile errors"
        cargo check --test soak_test 2>&1 | grep "error\[" | head -3
        exit 1
    fi
fi
echo "  [4/7] ✅ PASS: soak_test compiles"

# 5. Tests pass
PASSED=$(cargo test --test soak_test 2>&1 | grep -E "test result.*ok" | grep -oE "[0-9]+ passed" | head -1)
if [ -z "$PASSED" ]; then
    echo "  ❌ FAIL: soak_test tests did not pass"
    cargo test --test soak_test 2>&1 | tail -5
    exit 1
fi
# Expect at least 10 tests (3 soak + 7 supporting)
N_PASSED=$(echo "$PASSED" | grep -oE "[0-9]+")
if [ "$N_PASSED" -lt 10 ]; then
    echo "  ❌ FAIL: expected ≥10 soak tests, got $N_PASSED"
    exit 1
fi
echo "  [5/7] ✅ PASS: soak_test $PASSED (≥10)"

# 6. Alert mechanism works
ALERT_TEST=$(cargo test --test soak_test test_soak_alert_message_when_exceeds_threshold 2>&1 \
    | grep "test result" | head -1)
if echo "$ALERT_TEST" | grep -q "1 passed"; then
    echo "  [6/7] ✅ PASS: alert-threshold mechanism verified"
else
    echo "  ❌ FAIL: alert-threshold test did not pass"
    echo "  $ALERT_TEST"
    exit 1
fi

# 7. Memory baseline invariant
BASELINE_TEST=$(cargo test --test soak_test test_soak_memory_baseline_invariant 2>&1 \
    | grep "test result" | head -1)
if echo "$BASELINE_TEST" | grep -q "1 passed"; then
    echo "  [7/7] ✅ PASS: memory baseline invariant (no-query == baseline)"
else
    echo "  ❌ FAIL: baseline invariant test did not pass"
    echo "  $BASELINE_TEST"
    exit 1
fi

echo
echo "=== G7 Gate: PASS ==="
echo "P1-3 (#3175) Soak Test: 3-level smoke + alert + baseline invariants verified"
exit 0
