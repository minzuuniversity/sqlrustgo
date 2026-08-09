#!/usr/bin/env bash
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
