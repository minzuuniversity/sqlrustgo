#!/usr/bin/env bash
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
# v3.11.0 Alpha Gate — 进入 Alpha 阶段必须通过
#
# 2026-07-15 (DRAFT → ALPHA promotion)
#
# Per-version gate script following the v3.10.0 pattern (check_alpha_v3.10.0.sh).
#
# NOTE: This script does NOT use set -e — each check runs independently
# to produce a full report (matches check_alpha_v380.sh style).
#
# Required files (8): per STAGE.yaml ALPHA stage
#   - CHANGELOG.md
#   - RELEASE_NOTES.md
#   - ARCHITECTURE.md
#   - V311_ISSUES_PLAN.md
#   - DRAFT_ASSESSMENT_AND_ALPHA_GATE.md
#   - GA_GATE_REPORT.md
#   - POST_GA_PLAN.md
#   - RC_BLOCKERS_REPORT.md

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

PASS=0; TOTAL=0; BLOCKERS=0

check() {
    local label="$1"; local cmd="$2"
    ((TOTAL++))
    echo -n "  [$label] "
    if eval "$cmd" >/dev/null 2>&1; then
        echo "PASS"; ((PASS++))
    else
        echo "FAIL"; ((BLOCKERS++))
    fi
}

echo "=== v3.11.0 Alpha Gate ==="
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown') @ $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
echo ""

# ============================================================
# A1: Standard build/test/clippy/fmt checks
# ============================================================
echo "--- A1: Build/Test/Format ---"

check "A1_BUILD" "cargo build --all-features --quiet"
check "A1_TEST" "cargo test --all-features --lib --quiet"
check "A1_FMT" "cargo fmt --check --quiet"
# B3: clippy with -D warnings (ALPHA gate requirement)
check "A1_CLIPPY" "cargo clippy --all-features --workspace -- -D warnings"

# ============================================================
# A2: Architecture invariants
# ============================================================
echo ""
echo "--- A2: Architecture Invariants ---"

check "A2_ARCH_INVARIANTS" "bash scripts/gate/check_arch_invariants.sh"

# ============================================================
# A3: Required files (v3.11.0 specific)
# ============================================================
echo ""
echo "--- A3: Required Files (v3.11.0) ---"

check "A3_CHANGELOG" "test -f docs/releases/v3.11.0/CHANGELOG.md"
check "A3_RELEASE_NOTES" "test -f docs/releases/v3.11.0/RELEASE_NOTES.md"
check "A3_STAGE_YAML" "test -f docs/releases/v3.11.0/STAGE.yaml"
check "A3_VERSION_PLAN" "test -f docs/releases/v3.11.0/plans/V311_VERSION_PLAN.md"
check "A3_DEV_PLAN" "test -f docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md"
check "A3_ARCHITECTURE" "test -f docs/releases/v3.11.0/ARCHITECTURE.md"
check "A3_ISSUES_PLAN" "test -f docs/releases/v3.11.0/plans/V311_ISSUES_PLAN.md"
check "A3_DRAFT_ASSESSMENT" "test -f docs/releases/v3.11.0/DRAFT_ASSESSMENT_AND_ALPHA_GATE.md"

# ============================================================
# A4: Branch + state
# ============================================================
echo ""
echo "--- A4: Branch/State Sanity ---"

check "A4_BRANCH" "git rev-parse --abbrev-ref HEAD | grep -q '^develop/v3.11.0$'"

# ============================================================
# Summary
# ============================================================
echo ""
echo "=== v3.11.0 Alpha Gate Summary ==="
echo "PASS: $PASS/$TOTAL"
echo "BLOCKERS: $BLOCKERS"
echo ""

if [ "$BLOCKERS" -eq 0 ]; then
    echo "STATUS: ALPHA GATE PASS"
    exit 0
else
    echo "STATUS: ALPHA GATE FAIL — $BLOCKERS blocker(s)"
    exit 1
fi
