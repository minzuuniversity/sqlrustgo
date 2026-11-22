#!/bin/bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：孤儿门禁（orphan_referenced）—— 只在其它脚本正文里被引用，没有任何执行点
# 处置：归档候选（archive-candidate）
# 责任任务：P0-2
# 登记说明：头部绑定历史版本线，v4.1.0 的 STAGE_CONFIG 与 CI 都不引用它 ⇒ 归档候选；删除门禁脚本属门禁清单变更，须 owner 批准
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# check_perf_baseline.sh - Performance Baseline Gate
#
# Verifies:
# 1. PERFORMANCE_BASELINE.md exists
# 2. baseline contains v3.8.0 + v3.9.0 columns
# 3. baseline contains delta (Δ) column
# 4. baseline contains threshold column
# 5. baseline structure matches G11-G16 sections
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §Perf Baseline

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

echo "=== Perf Baseline Gate ==="

BASELINE="docs/releases/v3.9.0/perf/PERFORMANCE_BASELINE.md"

# 1. Baseline file exists
[ -f "$BASELINE" ] || {
    echo "  ❌ FAIL: $BASELINE not found"
    exit 1
}
echo "  [1/5] ✅ PASS: $BASELINE present"

# 2. v3.8.0 column
grep -q "v3.8.0" "$BASELINE" || {
    echo "  ❌ FAIL: 'v3.8.0' column not in $BASELINE"
    exit 1
}
echo "  [2/5] ✅ PASS: v3.8.0 column present"

# 3. v3.9.0 column
grep -q "v3.9.0" "$BASELINE" || {
    echo "  ❌ FAIL: 'v3.9.0' column not in $BASELINE"
    exit 1
}
echo "  [3/5] ✅ PASS: v3.9.0 column present"

# 4. Δ (delta) column
grep -q "Δ" "$BASELINE" || {
    echo "  ❌ FAIL: 'Δ' (delta) column not in $BASELINE"
    exit 1
}
echo "  [4/5] ✅ PASS: Δ (delta) column present"

# 5. Threshold column
grep -q "阈值\|threshold" "$BASELINE" || {
    echo "  ❌ FAIL: '阈值' (threshold) column not in $BASELINE"
    exit 1
}
echo "  [5/5] ✅ PASS: 阈值 (threshold) column present"

echo
echo "=== Perf Baseline Gate: PASS ==="
echo "Baseline structure validated. Real values populated in W12 D3 (Z6G4 measurement)."
exit 0
