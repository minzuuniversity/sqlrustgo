#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：孤儿门禁（orphan_referenced）—— 只在其它脚本正文里被引用，没有任何执行点
# 处置：需外部主机（needs-external-host）
# 责任任务：P0-2
# 登记说明：判据依赖仓外环境：Prometheus/metrics 采集端 ⇒ 无执行点即永不运行；要么挂上执行点并补环境，要么从能力声称里撤下
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# check_prometheus_metrics.sh — GA-P1 Prometheus Metrics Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 Prometheus Metrics Gate ==="
[ -f docs/prometheus-metrics.md ] && echo "  [PASS] prometheus-metrics.md" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
[ -f docs/slow-query-log.md ] && echo "  [PASS] slow-query-log.md" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_prometheus_metrics.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL] gate"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
