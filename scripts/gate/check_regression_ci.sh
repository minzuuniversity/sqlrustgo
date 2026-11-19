#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：孤儿门禁（orphan_referenced）—— 只在其它脚本正文里被引用，没有任何执行点
# 处置：待接入执行点（needs-execution-point）
# 责任任务：P0-2
# 登记说明：判据可在仓内自足执行，但不在任何阶段清单/CI 执行点 ⇒ P0-2 定义执行点后必须挂上，否则撤声称
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# check_regression_ci.sh — GA-P1 Regression Test Suite Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 Regression Test Suite Gate ==="
[ -f docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md ] && echo "  [PASS] REGRESSION_TEST_SUITE.md" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
grep -q "TPC-H" docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md && echo "  [PASS] TPC-H documented" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "Chaos Soak" docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md && echo "  [PASS] Chaos Soak" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "Upgrade" docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md && echo "  [PASS] Upgrade documented" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "Coverage" docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md && echo "  [PASS] Coverage documented" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_regression_ci.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
