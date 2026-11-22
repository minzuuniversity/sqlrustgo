#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：孤儿门禁（orphan_referenced）—— 只在其它脚本正文里被引用，没有任何执行点
# 处置：归档候选（archive-candidate）
# 责任任务：P0-2
# 登记说明：G4 属 v3.10.0 版本线(#3732)，v4.1.0 不引用 ⇒ 归档候选；TPC-H 基线判据在 v4.1.0 侧另需重建
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# scripts/gate/check_tpch_sf1.sh
#
# G4: TPC-H SF=1.0 gate for v3.10.0 / Issue #3732.
# Runs the canonical SF=1 baseline wrapper against official dbgen row counts.

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

SF1_DIR="${TPCH_SF1_DIR:-${SF1_DIR:-/tmp/tpch-sf1}}"

if [[ ${1:-} == "--run" ]]; then
    shift
elif [[ $# -eq 0 ]]; then
    set -- --dry-run
fi

exec bash scripts/tpch_sf1_baseline.sh --sf1-dir "$SF1_DIR" "$@"
