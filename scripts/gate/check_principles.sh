#!/usr/bin/env bash
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
