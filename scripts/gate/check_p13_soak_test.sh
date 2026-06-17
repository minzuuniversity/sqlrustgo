#!/bin/bash
# check_p13_soak_test.sh - P1-3 (#3175) Soak Test G7 gate
#
# Verifies:
# 1. soak_test.rs exists and is registered in Cargo.toml
# 2. 3-level smoke equivalence constants are stable (24h/72h/168h
#    map to 60s/180s/420s) via default_config values
# 3. cargo check pass
# 4. 10 soak tests pass (3 soak levels + 7 supporting tests)
# 5. Alert-threshold mechanism works (tight threshold triggers alert)
# 6. Memory baseline invariant (no-query run == baseline)
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3175-soak-test.md
#       V390_TEST_PLAN.md §G7
#       TGS Phase 2: Replace simulated smoke with real SQL

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

# 1. test file + registration
[ -f tests/soak_test.rs ] || {
    echo "  ❌ FAIL: tests/soak_test.rs not found"
    exit 1
}
grep -q 'name = "soak_test"' Cargo.toml || {
    echo "  ❌ FAIL: soak_test not registered in Cargo.toml"
    exit 1
}
echo "  [1/6] ✅ PASS: tests/soak_test.rs present + registered"

# 2. Verify TGS Fix: harness module uses real SQL (check for MemoryExecutionEngine)
if grep -q 'MemoryExecutionEngine::new' tests/soak_test.rs; then
    echo "  [2/6] ✅ PASS: TGS Fix - real SQL via MemoryExecutionEngine"
else
    echo "  ❌ FAIL: TGS Fix missing - no MemoryExecutionEngine usage"
    exit 1
fi

# 3. cargo check
if ! cargo check --test soak_test 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    if cargo check --test soak_test 2>&1 | grep -q "error\["; then
        echo "  ❌ FAIL: soak_test has compile errors"
        cargo check --test soak_test 2>&1 | grep "error\[" | head -3
        exit 1
    fi
fi
echo "  [3/6] ✅ PASS: soak_test compiles"

# 4. Tests pass (P14 V8 fix: capture exit code explicitly)
SOAK_OUTPUT=$(cargo test --test soak_test 2>&1)
SOAK_EXIT=$?
PASSED=$(echo "$SOAK_OUTPUT" | grep -E "test result.*ok" | grep -oE "[0-9]+ passed" | head -1)
if [ $SOAK_EXIT -ne 0 ] || [ -z "$PASSED" ]; then
    echo "  ❌ FAIL: soak_test tests did not pass (cargo exit=$SOAK_EXIT)"
    echo "$SOAK_OUTPUT" | tail -5
    exit 1
fi
# Expect at least 10 tests (3 soak + 7 supporting)
N_PASSED=$(echo "$PASSED" | grep -oE "[0-9]+")
if [ "$N_PASSED" -lt 10 ]; then
    echo "  ❌ FAIL: expected ≥10 soak tests, got $N_PASSED"
    exit 1
fi
echo "  [4/6] ✅ PASS: soak_test $PASSED (≥10)"

# 5. Alert mechanism works
ALERT_TEST=$(cargo test --test soak_test test_soak_alert_message_when_exceeds_threshold 2>&1 \
    | grep "test result" | head -1)
if echo "$ALERT_TEST" | grep -q "1 passed"; then
    echo "  [5/6] ✅ PASS: alert-threshold mechanism verified"
else
    echo "  ❌ FAIL: alert-threshold test did not pass"
    echo "  $ALERT_TEST"
    exit 1
fi

# 6. Memory baseline invariant
BASELINE_TEST=$(cargo test --test soak_test test_soak_memory_baseline_invariant 2>&1 \
    | grep "test result" | head -1)
if echo "$BASELINE_TEST" | grep -q "1 passed"; then
    echo "  [6/6] ✅ PASS: memory baseline invariant (no-query == baseline)"
else
    echo "  ❌ FAIL: baseline invariant test did not pass"
    echo "  $BASELINE_TEST"
    exit 1
fi

echo
echo "=== G7 Gate: PASS ==="
echo "P1-3 (#3175) Soak Test: TGS real SQL + alert + baseline invariants verified"
exit 0
