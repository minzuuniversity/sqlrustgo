#!/usr/bin/env bash
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
# =============================================================================
# G6 — V310-06 PR2 / Issue #3723 C-2 (INTERSECT / EXCEPT / UNION ORDER BY)
# Syntax & Compilation Gate
#
# Verifies:
#   C-2a  INTERSECT / INTERSECT ALL  (SQL-92 set operation)
#   C-2b  EXCEPT  / EXCEPT ALL       (SQL-92 set operation)
#   C-2c  UNION with trailing ORDER BY / LIMIT / OFFSET  (lifted onto UNION)
#
# Run: bash scripts/gate/check_g6_set_ops_syntax.sh
# =============================================================================
set -euo pipefail

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
NC=$'\033[0m'

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

pass=0
fail=0

run_check() {
    local label="$1"; shift
    local log=$(mktemp)
    if "$@" > "$log" 2>&1; then
        echo -e "[G6] $label ... ${GREEN}PASS${NC}"
        pass=$((pass + 1))
    else
        echo -e "[G6] $label ... ${RED}FAIL${NC}"
        cat "$log"
        fail=$((fail + 1))
    fi
    rm -f "$log"
}

echo "# G6 — V310-06 PR2 Set Operations Syntax Gate"
echo "# Issue #3723 C-2: INTERSECT / EXCEPT / UNION ORDER BY"
echo ""

run_check "cargo build -p sqlrustgo-parser" \
    sh -c '! cargo build -p sqlrustgo-parser 2>&1 | grep -q "^error"'

run_check "cargo test -p sqlrustgo-parser --lib (0 failures)" \
    sh -c 'cargo test -p sqlrustgo-parser --lib 2>&1 | grep -q "test result: ok. .*0 failed"'

run_check "cargo clippy --all-features -D warnings" \
    sh -c '! cargo clippy --all-features -- -D warnings 2>&1 | grep -q "^error"'

run_check "test: intersect_returns_common_rows_test" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- intersect_returns_common_rows_test 2>&1 | grep -q "test result: ok"'

run_check "test: except_returns_left_minus_right_test" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- except_returns_left_minus_right_test 2>&1 | grep -q "test result: ok"'

run_check "test: UNION ORDER BY LIMIT lifted" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- test_set_op_union_order_by_limit_lifted 2>&1 | grep -q "test result: ok"'

run_check "test: INTERSECT ALL parse" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- test_set_op_intersect_all 2>&1 | grep -q "test result: ok"'

run_check "test: EXCEPT ALL parse" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- test_set_op_except_all 2>&1 | grep -q "test result: ok"'

run_check "test: UNION LIMIT lifted" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- test_set_op_union_limit_lifted 2>&1 | grep -q "test result: ok"'

echo ""
echo "============================================"
if [ "$fail" -eq 0 ]; then
    echo -e "${GREEN}G6 PASSED — $pass/$pass checks${NC}"
    exit 0
else
    echo -e "${RED}G6 FAILED — $pass passed, $fail failed${NC}"
    exit 1
fi
