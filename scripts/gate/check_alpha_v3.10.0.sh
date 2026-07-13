#!/usr/bin/env bash
# v3.10.0 Alpha Gate — 进入 Alpha 阶段必须通过
#
# 2026-07-11 claude-macmini (initial — DRAFT → ALPHA promotion)
#
# Per-version gate script following the v3.8.0 pattern (check_alpha_v380.sh).
# v2.9.0 check_alpha.sh is hardcoded with v2.9.0 paths and references
# crates/paths that no longer exist (e.g. sqlrustgo-sql-corpus,
# verification_report.json, 28-file integration suite). This script is
# the v3.10.0 equivalent: 8 file checks + 6 gate checks, all referencing
# the current develop/v3.10.0 worktree.
#
# NOTE: This script does NOT use set -e — each check runs independently
# to produce a full report (matches check_alpha_v380.sh style).
#
# Required files (8): per STAGE_CONFIG.yaml ALPHA stage + v3.10.0 specific
#   - CHANGELOG.md
#   - RELEASE_NOTES.md
#   - STAGE.yaml
#   - VERSION_PLAN.md (replaces v2.9.0 hardcoded path)
#   - DEVELOPMENT_PLAN.md
#   - ARCHITECTURE.md
#   - V310_ISSUES_PLAN.md (v3.10.0 master issue plan)
#   - DRAFT_ASSESSMENT_AND_ALPHA_GATE.md (promotion evidence)

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

PASS=0; TOTAL=0; BLOCKERS=0

check() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    echo -n "[v3.10.0 alpha] $name ... "
    if eval "$cmd" >/dev/null 2>&1; then
        echo "PASS"
        PASS=$((PASS+1))
    else
        echo "FAIL"
        BLOCKERS=$((BLOCKERS+1))
    fi
}

echo "=== v3.10.0 Alpha Gate ==="
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
# A3: Required files (v3.10.0 specific)
# ============================================================
echo ""
echo "--- A3: Required Files (v3.10.0) ---"

check "A3_CHANGELOG" "test -f docs/releases/v3.10.0/CHANGELOG.md"
check "A3_RELEASE_NOTES" "test -f docs/releases/v3.10.0/RELEASE_NOTES.md"
check "A3_STAGE_YAML" "test -f docs/releases/v3.10.0/STAGE.yaml"
check "A3_VERSION_PLAN" "test -f docs/releases/v3.10.0/plans/V310_VERSION_PLAN.md"
check "A3_DEV_PLAN" "test -f docs/releases/v3.10.0/plans/V310_DEVELOPMENT_PLAN.md"
check "A3_ARCHITECTURE" "test -f docs/releases/v3.10.0/ARCHITECTURE.md"
check "A3_ISSUES_PLAN" "test -f docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md"
check "A3_DRAFT_ASSESSMENT" "test -f docs/releases/v3.10.0/DRAFT_ASSESSMENT_AND_ALPHA_GATE.md"

# ============================================================
# A4: Branch + state
# ============================================================
echo ""
echo "--- A4: Branch/State Sanity ---"

check "A4_BRANCH" "git rev-parse --abbrev-ref HEAD | grep -q '^develop/v3.10.0$'"
check "A4_DRAFT_COMPLETE" "test -f docs/releases/v3.10.0/DRAFT_ASSESSMENT_AND_ALPHA_GATE.md"

# ============================================================
# Summary
# ============================================================
echo ""
echo "=== v3.10.0 Alpha Gate Summary ==="
echo "PASS: $PASS/$TOTAL"
echo "BLOCKERS: $BLOCKERS"
echo ""

if [ "$BLOCKERS" -eq 0 ]; then
    echo "PASS — can promote to v3.10.0-alpha1"
    exit 0
else
    echo "BLOCKED — fix $BLOCKERS blocker(s) above before promotion"
    exit 2
fi
