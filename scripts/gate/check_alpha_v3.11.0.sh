#!/usr/bin/env bash
# v3.11.0 Alpha Gate — 进入 Alpha 阶段必须通过
#
# 2026-07-15 claude-macmini (initial — DRAFT → ALPHA promotion)
#
# Per-version gate script following the v3.8.0 pattern (check_alpha_v380.sh).
# v3.11.0 variant: 8 file checks + 6 gate checks, all referencing
# the current develop/v3.11.0 worktree.
#
# NOTE: Does NOT use set -e — each check runs independently for full report.
#
# Required files (8): per STAGE_CONFIG.yaml ALPHA stage + v3.11.0 specific
#   - CHANGELOG.md (root)
#   - docs/releases/v3.11.0/RELEASE_NOTES.md
#   - docs/releases/v3.11.0/STAGE.yaml
#   - docs/releases/v3.11.0/VERSION_PLAN.md
#   - docs/releases/v3.11.0/plans/V311_VERSION_PLAN.md
#   - docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md
#   - docs/releases/v3.11.0/plans/V311_DEBT_CLOSURE_PLAN.md
#   - docs/releases/v3.11.0/FEATURE_CHECKLIST.md

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
    [ -x "$HOME/.cargo/bin/cargo" ] && export PATH="$HOME/.cargo/bin:$PATH"
fi

PASS=0; TOTAL=0; BLOCKERS=0

check() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    if eval "$cmd" >/dev/null 2>&1; then
        PASS=$((PASS+1))
        printf "  [PASS] %s\n" "$name"
    else
        BLOCKERS=$((BLOCKERS+1))
        printf "  [FAIL] %s — please fix before promoting to ALPHA\n" "$name"
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
# Clippy is BETA-gate requirement (>=95% coverage), not ALPHA. Skipped here.

# ============================================================
# A2: Architecture invariants (C-ARCH-01~05)
# ============================================================
echo ""
echo "--- A2: Architecture Invariants ---"

check "A2_ARCH_INVARIANTS" "bash scripts/gate/check_arch_invariants.sh"
check "A2_ARCH3_NO_BYPASS" "bash scripts/gate/check_arch3_no_bypass.sh"

# ============================================================
# A3: Required files (v3.11.0 specific)
# ============================================================
echo ""
echo "--- A3: Required Files (v3.11.0) ---"

check "A3_CHANGELOG" "test -f CHANGELOG.md"
check "A3_RELEASE_NOTES" "test -f docs/releases/v3.11.0/RELEASE_NOTES.md"
check "A3_STAGE_YAML" "test -f docs/releases/v3.11.0/STAGE.yaml"
check "A3_VERSION_PLAN_ROOT" "test -f docs/releases/v3.11.0/VERSION_PLAN.md"
check "A3_VERSION_PLAN" "test -f docs/releases/v3.11.0/plans/V311_VERSION_PLAN.md"
check "A3_DEV_PLAN" "test -f docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md"
check "A3_DEBT_PLAN" "test -f docs/releases/v3.11.0/plans/V311_DEBT_CLOSURE_PLAN.md"
check "A3_FEATURE_CHECKLIST" "test -f docs/releases/v3.11.0/FEATURE_CHECKLIST.md"

# ============================================================
# A4: Branch + state
# ============================================================
echo ""
echo "--- A4: Branch/State Sanity ---"

check "A4_BRANCH" "git rev-parse --abbrev-ref HEAD | grep -q '^develop/v3.11.0$'"
check "A4_NO_UNCOMMITTED" "git diff --quiet && git diff --cached --quiet"

# ============================================================
# Summary
# ============================================================
echo ""
echo "=== v3.11.0 Alpha Gate Summary ==="
echo "PASS: $PASS/$TOTAL"
echo "BLOCKERS: $BLOCKERS"
echo ""

if [ "$BLOCKERS" -eq 0 ]; then
    echo "✓ All checks pass — ready for ALPHA promotion."
    exit 0
else
    echo "✗ $BLOCKERS blocker(s) must be resolved before ALPHA promotion."
    exit 1
fi
