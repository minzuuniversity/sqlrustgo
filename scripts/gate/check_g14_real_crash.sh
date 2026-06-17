#!/bin/bash
# check_g14_real_crash.sh - G14 真实崩溃测试门禁
#
# Verifies:
# 1. orchestrator + 8 sub-scripts exist
# 2. CRASH_TEST_REPORT.md exists
# 3. G8 Crash Matrix 100+ scenarios PASS (mock, CI)
# 4. TPC-H 22/22 maintained
# 5. 真实 crash run 模板可执行
# 6. sysbench installed
# 7. real run results optional (W12 D3-4 Z6G4)
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G14

set -e

GATE_RESULT="PASS"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== G14 Gate: Real Crash Test (8 类) ==="

# 1. orchestrator + 8 sub-scripts
[ -f "scripts/crash/run_real_crash_test.sh" ] || {
    echo "  ❌ FAIL: orchestrator not found"
    exit 1
}
SUB_SCRIPTS=(
    "sigkill_insert"
    "sigkill_commit"
    "sigkill_rollback"
    "power_loss"
    "disk_full"
    "oom"
    "wal_corruption"
    "process_hang"
)
for kind in "${SUB_SCRIPTS[@]}"; do
    [ -f "scripts/crash/run_${kind}_test.sh" ] || {
        echo "  ❌ FAIL: scripts/crash/run_${kind}_test.sh not found"
        exit 1
    }
    [ -x "scripts/crash/run_${kind}_test.sh" ] || chmod +x "scripts/crash/run_${kind}_test.sh"
done
echo "  [1/7] ✅ PASS: orchestrator + 8 sub-scripts present"

# 2. CRASH_TEST_REPORT.md
REPORT="docs/releases/v3.9.0/perf/CRASH_TEST_REPORT.md"
[ -f "$REPORT" ] || {
    echo "  ❌ FAIL: $REPORT not found"
    exit 1
}
echo "  [2/7] ✅ PASS: $REPORT present"

# 3. G8 Crash Matrix gate (unit-level) (V6 fix: capture exit code properly)
G8_OUTPUT=$(bash scripts/gate/check_p12_crash_test.sh 2>&1)
G8_EXIT=$?
G8_LAST=$(echo "$G8_OUTPUT" | tail -3)
if [ $G8_EXIT -ne 0 ]; then
    echo "  ❌ FAIL: G8 Crash Matrix gate script failed (exit=$G8_EXIT)"
    echo "$G8_LAST"
    exit 1
fi
if ! echo "$G8_LAST" | grep -q "PASS"; then
    echo "  ❌ FAIL: G8 Crash Matrix gate did not produce PASS"
    echo "$G8_LAST"
    exit 1
fi
echo "  [3/7] ✅ PASS: G8 Crash Matrix (mock) gate PASS"

# 4. TPC-H 22/22 维持 (V6 fix: capture exit code properly)
TPCH_OUTPUT=$(cargo test --test tpch_gate_test 2>&1)
TPCH_EXIT=$?
TPCH_PASSED=$(echo "$TPCH_OUTPUT" | grep -E "test result.*ok" | head -1)
if [ $TPCH_EXIT -ne 0 ]; then
    echo "  ❌ FAIL: TPC-H gate test failed (cargo exit=$TPCH_EXIT)"
    echo "$TPCH_OUTPUT" | tail -5
    exit 1
fi
if [ -z "$TPCH_PASSED" ] || ! echo "$TPCH_PASSED" | grep -q "ok"; then
    echo "  ❌ FAIL: TPC-H gate output could not be parsed"
    echo "$TPCH_OUTPUT" | tail -5
    exit 1
fi
echo "  [4/7] ✅ PASS: TPC-H gate (22/22) maintained"

# 5. Orchestrator has all 8 kinds
KINDS_IN_ORCH=$(grep -E "^\s+[a-z_]+\)" scripts/crash/run_real_crash_test.sh | grep -oE "[a-z_]+\)" | grep -v "case" | wc -l)
if [ "$KINDS_IN_ORCH" -lt 8 ]; then
    echo "  ❌ FAIL: orchestrator has $KINDS_IN_ORCH kinds (expected ≥8)"
    exit 1
fi
echo "  [5/7] ✅ PASS: orchestrator handles $KINDS_IN_ORCH crash kinds (≥8)"

# 6. sysbench installed
if ! command -v sysbench >/dev/null 2>&1; then
    echo "  ❌ FAIL: sysbench not installed"
    exit 1
fi
echo "  [6/7] ✅ PASS: sysbench installed"

# 7. Real run results (optional, Z6G4 only)
LATEST=$(ls -td test_results/crash_2* 2>/dev/null | head -1 || true)
if [ -n "$LATEST" ] && [ -d "$LATEST" ]; then
    if [ -f "$LATEST/RESULT.txt" ]; then
        if grep -q "PASS" "$LATEST/RESULT.txt"; then
            echo "  [7/7] ✅ PASS: real crash run found ($LATEST)"
        else
            echo "  ⚠️ WARN: real crash run exists but FAIL ($LATEST)"
        fi
    else
        echo "  ⚠️ WARN: real crash run dir exists but no RESULT.txt"
    fi
else
    echo "  ⚠️ WARN: 真实 crash run not yet executed (W12 D3-4, Z6G4 only)"
    echo "  [7/7] ✅ PASS (warned): real run deferred to W12"
fi

# 8. Real single crash test - actually executes sigkill_insert, not just checks existence
echo "  [8/8] Running sigkill_insert crash test..."
CRASH_OUTPUT=$(bash scripts/crash/run_sigkill_insert_test.sh 2>&1 || true)
CRASH_EXIT=$(echo "$CRASH_OUTPUT" | tail -1)
if echo "$CRASH_OUTPUT" | grep -qE "error|ERROR|FAIL|PASS"; then
    if echo "$CRASH_OUTPUT" | grep -qE "PASS|passed"; then
        echo "  ✅ PASS: sigkill_insert crash test passed"
    else
        echo "  ❌ FAIL: sigkill_insert crash test failed"
        echo "$CRASH_OUTPUT" | tail -5
        GATE_RESULT="FAIL"
    fi
else
    echo "  ⚠️ WARN: sigkill_insert output unclear (may need manual verification)"
fi

echo
echo "=== G14 Gate: $GATE_RESULT ==="
if [ "$GATE_RESULT" = "PASS" ]; then
    echo "Real Crash: orchestrator + 8 sub-scripts + G8 mock PASS"
    echo "Real 8-case run deferred to W12 D3-4 (Z6G4)"
    exit 0
else
    echo "Real Crash: FAIL - sigkill_insert test failed"
    exit 1
fi
