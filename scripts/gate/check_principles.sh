#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：待接入执行点（needs-execution-point）
# 责任任务：P0-4
# 登记说明：它是 5/10-principles 的合并入口(wrapper)，自己却没有执行点；而被它头部标为 DEPRECATED 的 check_10_principles_v310.sh 仍在 CI 跑 ⇒ 同一判据两份实现并存
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# =============================================================================
# check_principles.sh — Consolidated 5 + 10 Principles Gate (Phase 4)
# =============================================================================
# 2026-07-13 claude-macmini (Phase 4 implementation per DeepSeek feedback)
#
# Purpose: Single entry point to invoke both 5-principles and 10-principles
#          gate scripts (both DEPRECATED as standalone but still informative
#          per their own header comments). This wrapper preserves the
#          historical scripts in scripts/gate/ and provides one pass/fail
#          summary.
#
# Behavior:
#   - If both legacy scripts exist → run both, report results
#   - Either failing → exit 1
#   - Either missing → WARN (not fail, scripts/gate/README.md may have
#     moved them to legacy/)
#
# Usage:
#   bash scripts/gate/check_principles.sh
#   bash scripts/gate/check_principles.sh v3.10.0 /tmp/gate_reports
#
# Exit codes:
#   0 = both 5 + 10 principles PASS (or both missing)
#   1 = at least one FAIL
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

VERSION="${1:-v3.10.0}"
OUT_DIR="${2:-/tmp/principles-gate-$(date +%Y%m%d_%H%M%S)}"

mkdir -p "$OUT_DIR"

PASS=0; FAIL=0; SKIP=0
declare -a RESULTS

run_principle() {
    local label="$1" script="$2"
    # Prefer the legacy unversioned script; fall back to versioned v310.
    local ver_script="${script%.sh}_v310.sh"
    local found=""
    if [ -f "$REPO_ROOT/$script" ]; then
        found="$script"
    elif [ -f "$REPO_ROOT/$ver_script" ]; then
        found="$ver_script"
        script="$ver_script"
    fi
    if [ -z "$found" ]; then
        SKIP=$((SKIP+1))
        RESULTS+=("SKIP|$label|$script (not found)")
        printf "  [SKIP] %-30s %s (not present, may be in legacy/)\n" "$label" "$script"
        return 0
    fi
    if bash "$REPO_ROOT/$script" "$VERSION" "$OUT_DIR" >/dev/null 2>&1; then
        PASS=$((PASS+1))
        RESULTS+=("PASS|$label|$script")
        printf "  [PASS] %-30s %s\n" "$label" "$script"
    else
        FAIL=$((FAIL+1))
        RESULTS+=("FAIL|$label|$script")
        printf "  [FAIL] %-30s %s\n" "$label" "$script"
    fi
}

echo "=== Principles Gate (5 + 10) ==="
echo "version: $VERSION"
echo "out_dir: $OUT_DIR"
echo ""

run_principle "5-PRINCIPLES" "scripts/gate/check_5_principles.sh"
run_principle "10-PRINCIPLES" "scripts/gate/check_10_principles.sh"

echo ""
echo "=== Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo "SKIP: $SKIP"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "  → check_principles: PASS (or SKIP-only)"
    exit 0
else
    echo "  → check_principles: FAIL ($FAIL failed)"
    exit 1
fi
