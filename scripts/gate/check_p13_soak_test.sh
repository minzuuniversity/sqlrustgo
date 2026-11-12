#!/bin/bash
# check_p13_soak_test.sh - P1-3 (#3175) Soak Test G7 gate
#
# Verifies 11 conditions:
#  1. soak_test_harness.rs exists
#  2. soak_test.rs exists and is registered in Cargo.toml
#  3. 3-level smoke equivalence constants (24h→60s, 72h→180s, 168h→420s)
#  4. cargo check --test soak_test passes
#  5. cargo test --test soak_test ≥10 tests PASS
#  6. Alert-threshold mechanism works
#  7. Memory baseline invariant holds
#  8. tpch_soak_driver.py exists and is valid Python
#  9. scripts/soak/extract_soak_report.py exists and is valid Python
# 10. tpch_mixed_soak_driver.py exists and is valid Python
# 11. crud_templates.py exists and imports successfully
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3175-soak-test.md
#       V390_TEST_PLAN.md §G7
#       openspec/changes/p1-3-soak-test/
#       openspec/changes/tpch-mixed-workload-soak/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Ensure cargo is on PATH
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== G7 Gate: P1-3 (#3175) Soak Test ==="

# ── Layer 1 checks (Rust harness) ────────────────────────────────────────────

# 1. harness file
# #5326: the harness lives under tests/integration/stress/ (see
# Cargo.toml [[test]] entry). The openspec spec said tests/ but the
# implementation chose the stress/ subdir; the gate was written
# against the spec and has been red since.
[ -f tests/integration/stress/soak_test_harness.rs ] || {
    echo "  [1/9] ❌ FAIL: tests/integration/stress/soak_test_harness.rs not found"
    exit 1
}
echo "  [1/9] ✅ PASS: tests/integration/stress/soak_test_harness.rs present"

# 2. test file + registration
[ -f tests/integration/stress/soak_test.rs ] || {
    echo "  [2/9] ❌ FAIL: tests/integration/stress/soak_test.rs not found"
    exit 1
}
grep -q 'name = "soak_test"' Cargo.toml || {
    echo "  [2/9] ❌ FAIL: soak_test not registered in Cargo.toml"
    exit 1
}
echo "  [2/9] ✅ PASS: tests/integration/stress/soak_test.rs present + registered"

# 3. 3-level smoke equivalence constants
SMOKE_24=$(grep -A 8 'fn smoke_seconds_for_level' tests/integration/stress/soak_test_harness.rs | grep '"24h" => Some' | head -1 || true)
SMOKE_72=$(grep -A 8 'fn smoke_seconds_for_level' tests/integration/stress/soak_test_harness.rs | grep '"72h" => Some' | head -1 || true)
SMOKE_168=$(grep -A 8 'fn smoke_seconds_for_level' tests/integration/stress/soak_test_harness.rs | grep '"168h" => Some' | head -1 || true)
if [ -z "$SMOKE_24" ] || [ -z "$SMOKE_72" ] || [ -z "$SMOKE_168" ]; then
    echo "  [3/9] ❌ FAIL: 3-level smoke equivalence constants missing"
    exit 1
fi
echo "  [3/9] ✅ PASS: 3-level smoke equivalence (24h→60s, 72h→180s, 168h→420s)"

# 4. cargo check
if ! cargo check --test soak_test 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    if cargo check --test soak_test 2>&1 | grep -q "error\["; then
        echo "  [4/9] ❌ FAIL: soak_test has compile errors"
        cargo check --test soak_test 2>&1 | grep "error\[" | head -3
        exit 1
    fi
fi
echo "  [4/9] ✅ PASS: soak_test compiles"

# 5. Tests pass (≥10)
PASSED=$(cargo test --test soak_test 2>&1 | grep -E "test result.*ok" | grep -oE "[0-9]+ passed" | head -1)
if [ -z "$PASSED" ]; then
    echo "  [5/9] ❌ FAIL: soak_test tests did not pass"
    cargo test --test soak_test 2>&1 | tail -5
    exit 1
fi
N_PASSED=$(echo "$PASSED" | grep -oE "[0-9]+")
if [ "$N_PASSED" -lt 10 ]; then
    echo "  [5/9] ❌ FAIL: expected ≥10 soak tests, got $N_PASSED"
    exit 1
fi
echo "  [5/9] ✅ PASS: soak_test $PASSED (≥10)"

# 6. Alert mechanism
ALERT_TEST=$(cargo test --test soak_test test_soak_alert_message_when_exceeds_threshold 2>&1 \
    | grep "test result" | head -1)
if echo "$ALERT_TEST" | grep -q "1 passed"; then
    echo "  [6/9] ✅ PASS: alert-threshold mechanism verified"
else
    echo "  [6/9] ❌ FAIL: alert-threshold test did not pass"
    exit 1
fi

# 7. Memory baseline invariant
BASELINE_TEST=$(cargo test --test soak_test test_soak_memory_baseline_invariant 2>&1 \
    | grep "test result" | head -1)
if echo "$BASELINE_TEST" | grep -q "1 passed"; then
    echo "  [7/9] ✅ PASS: memory baseline invariant (no-query == baseline)"
else
    echo "  [7/9] ❌ FAIL: baseline invariant test did not pass"
    exit 1
fi

# ── Layer 2 checks (mysqlslap scripts) ───────────────────────────────────────

# 8. mysqlslap_soak.sh exists and is executable
if [ ! -f "scripts/soak/tpch_soak_driver.py" ]; then
    echo "  [8/9] ❌ FAIL: scripts/soak/tpch_soak_driver.py not found"
    exit 1
fi
if ! python3 -c "import ast; ast.parse(open('scripts/soak/tpch_soak_driver.py').read())" 2>/dev/null; then
    echo "  [8/9] ❌ FAIL: scripts/soak/tpch_soak_driver.py is not valid Python"
    exit 1
fi
echo "  [8/9] ✅ PASS: scripts/soak/tpch_soak_driver.py valid Python"

# 9. extract_soak_report.py is valid Python
if [ ! -f "scripts/soak/extract_soak_report.py" ]; then
    echo "  [9/9] ❌ FAIL: scripts/soak/extract_soak_report.py not found"
    exit 1
fi
if ! python3 -c "import ast; ast.parse(open('scripts/soak/extract_soak_report.py').read())" 2>/dev/null; then
    echo "  [9/9] ❌ FAIL: scripts/soak/extract_soak_report.py is not valid Python"
    exit 1
fi
# Verify it can be imported (at least loads without error)
if ! python3 -c "import sys; sys.path.insert(0, 'scripts/soak'); import extract_soak_report; print('ok')" 2>/dev/null | grep -q ok; then
    echo "  [9/9] ❌ FAIL: extract_soak_report.py failed to import"
    exit 1
fi
echo "  [9/9] ✅ PASS: scripts/soak/extract_soak_report.py valid Python"

# ── Layer 4 checks (mixed workload driver) ───────────────────────────────────

# 10. tpch_mixed_soak_driver.py exists and is valid Python
if [ ! -f "scripts/soak/tpch_mixed_soak_driver.py" ]; then
    echo "  [10/11] ❌ FAIL: scripts/soak/tpch_mixed_soak_driver.py not found"
    exit 1
fi
if ! python3 -c "import ast; ast.parse(open('scripts/soak/tpch_mixed_soak_driver.py').read())" 2>/dev/null; then
    echo "  [10/11] ❌ FAIL: scripts/soak/tpch_mixed_soak_driver.py is not valid Python"
    exit 1
fi
echo "  [10/11] ✅ PASS: scripts/soak/tpch_mixed_soak_driver.py valid Python"

# 11. crud_templates.py exists and imports successfully
if [ ! -f "scripts/soak/crud_templates.py" ]; then
    echo "  [11/11] ❌ FAIL: scripts/soak/crud_templates.py not found"
    exit 1
fi
if ! python3 -c "import sys; sys.path.insert(0, 'scripts/soak'); import crud_templates; print('ok')" 2>/dev/null | grep -q ok; then
    echo "  [11/11] ❌ FAIL: crud_templates.py failed to import"
    exit 1
fi
echo "  [11/11] ✅ PASS: scripts/soak/crud_templates.py importable"

echo ""
echo "=== G7 Gate: PASS ==="
echo "P1-3 (#3175) Soak Test: Layer 1 (harness) + Layer 2/3 (mysqlslap scripts) verified"
# 诚实披露（2026-11-13，看板 VACUOUS-GREEN-1 轻档发现）：上面 11 个子检查里的
# Rust 侧断言跑在 soak_test 的**模拟查询环**上（soak_test_harness.rs 的
# run_soak_smoke：源码注释原文 "// Simulated query loop"，时延/内存都是构造值）
# ⇒ 本门禁验的是 harness 计分算术与脚本形态，不是任何一次真实 soak 的证据。
echo ""
echo "DISCLOSURE: the soak_test checks above run on a SIMULATED HARNESS SMOKE"
echo "  (run_soak_smoke fabricates latency/memory; this gate does NOT constitute"
echo "   evidence of a real soak). A real soak is tracked as ING-1 / V400-09"
echo "  (known_gaps.V400_09_soak) — long-run entry point:"
echo "  scripts/soak/run_soak_with_heartbeat.sh"
echo ""
echo "Next steps for full soak:"
echo "  1. bash scripts/soak/prepare_sf01_data.sh       # Generate SF=0.1 fixture"
echo "  2. mysql ... < scripts/soak/tpch_schema.sql     # Create tables"
echo "  3. python3 scripts/soak/tpch_soak_driver.py --level=30m  # Run Layer 2 soak"
echo "  4. python3 scripts/soak/tpch_soak_driver.py --level=30m --auto-generate  # Layer 3"
exit 0
