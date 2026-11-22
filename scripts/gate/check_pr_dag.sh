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
# check_pr_dag.sh — alias for verify_pr_dag.sh (issue #5457)
# =============================================================================
# Purpose: Provide the gate under the SECOND name the repository already
#          refers to, so that both citations resolve to a real script.
# Coverage: GATE_CONDITIONS.md BE5 / B-F5 (cites verify_pr_dag.sh)
#           docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md:92
#           docs/releases/v3.8.0/alpha/ALPHA_STAGE_REVIEW.md:81
#           (both cite check_pr_dag.sh)
# Principle: P1 (有计划必有实现 — a cited artifact must exist)
# Verify:   execs verify_pr_dag.sh, forwarding all arguments and the exit code.
#
# Why two names:
#   The repository refers to this gate under two different names:
#     docs/governance/GATE_CONDITIONS.md:103  -> scripts/gate/verify_pr_dag.sh
#     docs/governance/GATE_CONDITIONS.md:129  -> scripts/gate/verify_pr_dag.sh
#     docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md:92
#                                           -> scripts/gate/check_pr_dag.sh
#     docs/releases/v3.8.0/alpha/ALPHA_STAGE_REVIEW.md:81
#                                           -> scripts/gate/check_pr_dag.sh
#   Both were missing. This PR lands ONE implementation and resolves BOTH
#   citations, rather than leaving one of them broken.
#
# Exit codes: identical to verify_pr_dag.sh (0 = PASS, 1 = FAIL, 2 = DRIFT).
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="$SCRIPT_DIR/verify_pr_dag.sh"

if [ ! -f "$TARGET" ]; then
    echo "❌ $TARGET not found — the alias cannot delegate."
    echo "   Cannot judge — DRIFT, not PASS."
    exit 2
fi

exec bash "$TARGET" "$@"
