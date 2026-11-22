#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：归档候选（archive-candidate）
# 责任任务：P0-4
# 登记说明：它是阶段门禁的模板(不是判据本身)，放在 scripts/gate/ 里虚增门禁分母 ⇒ 迁到 docs/templates，属门禁清单变更须 owner 批准
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# =============================================================================
# check_stage_template.sh — Stage Gate Template (Phase 4)
# =============================================================================
# 2026-07-13 claude-macmini (Phase 4 implementation per DeepSeek feedback)
#
# Purpose: Template that can be invoked as a generic stage-gate template
#          (VERSION variable) instead of hard-coded v3.10.0 in each per-version
#          script. Following the pattern in STAGE_CONFIG.yaml, per-version
#          gates are recommended (e.g. check_alpha_v3.10.0.sh, check_beta_gate.sh,
#          check_rc_gate_v3.10.0.sh), but this template provides a fallback
#          when a per-version script doesn't exist for a given version.
#
# Usage:
#   bash scripts/gate/check_stage_template.sh                       # default v3.10.0
#   VERSION=v3.11.0 bash scripts/gate/check_stage_template.sh       # explicit
#
# This is NOT a replacement for per-version gates. It is a SAFETY NET
# for the v3.10.0 → v3.11.0 transition period when not every version has
# a dedicated gate. Once a version has its own *_v3.X.Y.sh, prefer that.
#
# Exit codes:
#   0  = all checks PASS
#   1  = any check FAIL
#   2  = DRIFT (some ignored, no failures)
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

VERSION="${VERSION:-v3.10.0}"
VER_SHORT="${VERSION#v}"  # e.g. v3.10.0 -> 3.10.0

if ! command -v cargo >/dev/null 2>&1; then
    [ -x "$HOME/.cargo/bin/cargo" ] && export PATH="$HOME/.cargo/bin:$PATH"
fi

PASS=0; FAIL=0; TOTAL=0

check() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    if eval "$cmd" >/dev/null 2>&1; then
        PASS=$((PASS+1))
        printf "  [PASS] %s\n" "$name"
    else
        FAIL=$((FAIL+1))
        printf "  [FAIL] %s\n" "$name"
    fi
}

echo "=== Stage Gate Template (VERSION=$VERSION) ==="
echo ""

# ============================================================
# Per-version required files (per STAGE_CONFIG.yaml)
# ============================================================
echo "--- Required Files ---"
check "${VERSION} CHANGELOG" "test -f docs/releases/$VERSION/CHANGELOG.md"
check "${VERSION} RELEASE_NOTES" "test -f docs/releases/$VERSION/RELEASE_NOTES.md"
check "${VERSION} STAGE.yaml" "test -f docs/releases/$VERSION/STAGE.yaml"
check "${VERSION} ARCHITECTURE.md" "test -f docs/releases/$VERSION/ARCHITECTURE.md"

# Per-stage required files (auto-detect current stage from STAGE.yaml)
if [ -f "docs/releases/$VERSION/STAGE.yaml" ]; then
    current_stage=$(grep -E "current_stage:" "docs/releases/$VERSION/STAGE.yaml" | head -1 | awk '{print $2}' | tr -d '"' | tr -d "'")
    echo "Current stage: $current_stage"
    case "$current_stage" in
        BETA)
            check "${VERSION} TEST_PLAN.md" "test -f docs/releases/$VERSION/TEST_PLAN.md"
            check "${VERSION} FEATURE_CHECKLIST.md" "test -f docs/releases/$VERSION/FEATURE_CHECKLIST.md"
            check "STAGE_CONFIG.yaml" "test -f docs/governance/STAGE_CONFIG.yaml"
            ;;
        RC|GA)
            check "${VERSION} TEST_PLAN.md" "test -f docs/releases/$VERSION/TEST_PLAN.md"
            check "${VERSION} FEATURE_CHECKLIST.md" "test -f docs/releases/$VERSION/FEATURE_CHECKLIST.md"
            check "${VERSION} CHANGELOG.md" "test -f docs/releases/$VERSION/CHANGELOG.md"
            check "${VERSION} GA_GATE_REPORT.md" "test -f docs/releases/$VERSION/GA_GATE_REPORT.md"
            ;;
    esac
fi
echo ""

# ============================================================
# Per-version per-stage gate preference
# ============================================================
echo "--- Per-version Stage Gates ---"
if [ -n "$current_stage" ]; then
    case "$current_stage" in
        DRAFT) gate_script="check_docs_links.sh" ;;
        ALPHA) gate_script="check_alpha_v${VER_SHORT}.sh" ;;
        BETA)  gate_script="check_beta_gate.sh" ;;
        RC)    gate_script="check_rc_gate_v${VER_SHORT}.sh" ;;
        GA)    gate_script="check_rc_ga_gate.sh" ;;
        *)     gate_script="" ;;
    esac
    if [ -n "$gate_script" ] && [ -x "scripts/gate/$gate_script" ]; then
        check "STAGE_GATE ($current_stage -> $gate_script)" "bash scripts/gate/$gate_script"
    else
        printf "  [WARN] No per-version gate for %s/%s\n" "$VERSION" "$current_stage"
    fi
fi
echo ""

# ============================================================
# Universal gates
# ============================================================
echo "--- Universal Gates ---"
check "check_arch_invariants.sh" "bash scripts/gate/check_arch_invariants.sh"
check "check_arch3_no_bypass.sh" "bash scripts/gate/check_arch3_no_bypass.sh"
echo ""

# ============================================================
# Cargo build
# ============================================================
echo "--- Cargo Build ---"
check "cargo build --all-features" "cargo build --all-features --quiet"
check "cargo fmt --check" "cargo fmt --check --quiet"
echo ""

# ============================================================
# Summary
# ============================================================
echo "=== Summary ==="
echo "PASS:  $PASS / $TOTAL"
echo "FAIL:  $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "  → Stage gate template for $VERSION: PASS"
    exit 0
else
    echo "  → Stage gate template for $VERSION: FAIL"
    exit 1
fi
