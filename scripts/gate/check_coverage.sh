#!/bin/bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：已被取代（superseded）
# 责任任务：P0-4
# 登记说明：文件头自标 DEPRECATED 并点名取代者，属重复实现 ⇒ 归档候选(与 check_coverage_v312.sh 归一，须 owner 批准)
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# DEPRECATED 2026-08-10: use check_coverage_v312.sh (G17)
# This script is retained only to surface a clear error.
echo "ERROR: check_coverage.sh is deprecated (was v3.7.0 LEGACY)." >&2
echo "       Use check_coverage_v312.sh instead (see GATE_CONDITIONS.md G17)." >&2
exit 2
