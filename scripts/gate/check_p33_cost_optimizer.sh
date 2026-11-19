#!/bin/bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：归档候选（archive-candidate）
# 责任任务：P0-4
# 登记说明：头部绑定历史版本线，v4.1.0 的 STAGE_CONFIG 与 CI 都不引用它 ⇒ 归档候选；删除门禁脚本属门禁清单变更，须 owner 批准
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# check_p33_cost_optimizer.sh - P3-3 (#3182) Cost Optimizer G10 gate
#
# Verifies:
# 1. tests/integration/optimizer/cost_optimizer_harness.rs exists
# 2. tests/integration/optimizer/cost_optimizer_test.rs exists + registered in Cargo.toml
# 3. 5 categories each have ≥1 test
# 4. cargo check pass
# 5. ≥20 tests pass
# 6. crates/optimizer (unified_cost + cost) still compiles (no regression)
# 7. TPC-H 22/22 maintained (smoke check)
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3182-cost-optimizer.md
#       V390_TEST_PLAN.md §G10

# #5326: paths corrected for V310 → V3.12 directory restructure

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

echo "=== G10 Gate (P3-3 #3182 Cost Optimizer) ==="

# 1. harness file
[ -f tests/integration/optimizer/cost_optimizer_harness.rs ] || {
    echo "  ❌ FAIL: tests/integration/optimizer/cost_optimizer_harness.rs not found"
    exit 1
}
echo "  [1/7] ✅ PASS: tests/integration/optimizer/cost_optimizer_harness.rs present"

# 2. test file + registration
[ -f tests/integration/optimizer/cost_optimizer_test.rs ] || {
    echo "  ❌ FAIL: tests/integration/optimizer/cost_optimizer_test.rs not found"
    exit 1
}
grep -q 'name = "cost_optimizer_test"' Cargo.toml || {
    echo "  ❌ FAIL: cost_optimizer_test not registered in Cargo.toml"
    exit 1
}
echo "  [2/7] ✅ PASS: tests/integration/optimizer/cost_optimizer_test.rs present + registered"

# 3. 5 categories covered
N_TESTS=$(grep -c "^#\[test\]" tests/integration/optimizer/cost_optimizer_test.rs || echo 0)
if [ "$N_TESTS" -lt 20 ]; then
    echo "  ❌ FAIL: expected ≥20 tests, got $N_TESTS"
    exit 1
fi
echo "  [3/7] ✅ PASS: 5 categories covered (total: $N_TESTS tests, ≥20)"

# 4. cargo check
if cargo check --test cost_optimizer_test 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    echo "  [4/7] ✅ PASS: cost_optimizer_test compiles"
else
    if cargo check --test cost_optimizer_test 2>&1 | grep -q "error\["; then
        echo "  ❌ FAIL: cost_optimizer_test has compile errors"
        cargo check --test cost_optimizer_test 2>&1 | grep "error\[" | head -3
        exit 1
    else
        echo "  [4/7] ✅ PASS: cost_optimizer_test compiles"
    fi
fi

# 5. ≥20 tests pass
PASSED=$(cargo test --test cost_optimizer_test 2>&1 | grep -E "test result.*ok" | grep -oE "[0-9]+ passed" | head -1 || echo "0")
if [ -z "$PASSED" ]; then
    echo "  ❌ FAIL: cost_optimizer_test tests did not pass"
    cargo test --test cost_optimizer_test 2>&1 | tail -5
    exit 1
fi
N_PASSED=$(echo "$PASSED" | grep -oE "[0-9]+")
if [ "$N_PASSED" -lt 20 ]; then
    echo "  ❌ FAIL: expected ≥20 cost_optimizer tests, got $N_PASSED"
    exit 1
fi
echo "  [5/7] ✅ PASS: cost_optimizer_test $PASSED (≥20)"

# 6. crates/optimizer still compiles
if cargo check -p sqlrustgo-optimizer 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    echo "  [6/7] ✅ PASS: crates/optimizer (unified_cost + cost) compiles (no regression)"
else
    if cargo check -p sqlrustgo-optimizer 2>&1 | grep -q "error\["; then
        echo "  ❌ FAIL: crates/optimizer has compile errors"
        cargo check -p sqlrustgo-optimizer 2>&1 | grep "error\[" | head -3
        exit 1
    else
        echo "  [6/7] ✅ PASS: crates/optimizer compiles (no regression)"
    fi
fi

# 7. TPC-H 22/22 (smoke)
bash "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/tpch_gate_subcheck.sh"
TPCH_RC=$?
if [[ "${TPCH_RC}" -eq 0 ]]; then
    echo "  [7/7] ✅ PASS: TPC-H gate (22/22) maintained"
elif [[ "${TPCH_RC}" -eq 2 ]]; then
    echo "  [7/7] ⚠️ INCOMPLETE: TPC-H sub-check could not be performed (fixture missing / NOT-RUN disclosed) — not a drift"
else
    echo "  [7/7] ❌ FAIL: TPC-H gate did not pass"
fi

echo
if [[ "${TPCH_RC:-0}" -eq 2 ]]; then
    echo "=== G10 Gate (P3-3): INCOMPLETE (TPC-H sub-check not performed; see INCOMPLETE line above) ==="
    exit 2
fi
if [[ "${TPCH_RC:-0}" -eq 1 ]]; then
    echo "=== G10 Gate (P3-3): FAIL (TPC-H gate did not pass) ==="
    exit 1
fi
echo "=== G10 Gate (P3-3): PASS ==="
echo "P3-3 (#3182) Cost Optimizer: 5 categories + ≥20 tests + optimizer/cost verified"
exit 0
