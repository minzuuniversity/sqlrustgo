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
# check_admin_enhance.sh — GA-P1 Admin Enhancement Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 Admin Enhancement Gate ==="
[ -f scripts/admin/admin-status.sh ] && echo "  [PASS] admin-status.sh" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
[ -f scripts/admin/admin-flush-logs.sh ] && echo "  [PASS] admin-flush-logs.sh" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
bash -n scripts/admin/admin-status.sh 2>/dev/null && echo "  [PASS] status syntax" && PASS=$((PASS+1)) || { echo "  [FAIL] syntax"; FAIL=$((FAIL+1)); }
bash -n scripts/admin/admin-flush-logs.sh 2>/dev/null && echo "  [PASS] flush syntax" && PASS=$((PASS+1)) || { echo "  [FAIL] syntax"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_admin_enhance.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL] gate"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
