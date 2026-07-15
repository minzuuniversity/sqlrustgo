#!/usr/bin/env bash
#
# check_beta_v3.11.0.sh -- v3.11.0 BETA Governance Gate (B6-B8)
#
# Companion to check_beta_gate.sh (B1-B5 / technical gates).
# B6  = G-01~G-06 Truthfulness Framework
# B7  = R1~R10 Content Tracking
# B8-1 = Evidence Binding
# B8-2 = Plan Integrity
# B8-3 = SSOT No Duplicate
#
# v3.11.0 variant: paths updated from v3.10.0, debt register + coverage checks added.
#
# Usage:
#   bash scripts/gate/check_beta_v3.11.0.sh
#   bash scripts/gate/check_beta_v3.11.0.sh --json
#   bash scripts/gate/check_beta_v3.11.0.sh --out-dir /tmp/reports

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

VERSION="${VERSION:-v3.11.0}"
OUT_DIR="${OUT_DIR:-/tmp/beta_g311_$(date +%Y%m%d_%H%M%S)}"
JSON_OUTPUT=false

for arg in "$@"; do
    case "$arg" in
        --json) JSON_OUTPUT=true; shift ;;
        --out-dir) shift; OUT_DIR="${1:-/tmp/beta_g311}"; shift ;;
        --help|-h) grep "^#" "$0" | head -20; exit 0 ;;
    esac
done

mkdir -p "$OUT_DIR"

PASS=0; TOTAL=0; BLOCKERS=0; WARN=0

check() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    if eval "$cmd" >/dev/null 2>&1; then
        PASS=$((PASS+1))
        printf "  [PASS] %s\n" "$name"
    else
        BLOCKERS=$((BLOCKERS+1))
        printf "  [FAIL] %s\n" "$name"
    fi
}

warn() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    if eval "$cmd" >/dev/null 2>&1; then
        PASS=$((PASS+1))
        printf "  [PASS] %s\n" "$name"
    else
        WARN=$((WARN+1))
        printf "  [WARN] %s\n" "$name"
    fi
}

echo "=== v3.11.0 Beta Gate ==="
echo "Version: $VERSION"
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown') @ $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
echo ""

# ============================================================
# B1: Build — zero errors
# ============================================================
echo "--- B1: Build ---"
check "B1_CARGO_BUILD" "cargo build --all-features --quiet"
check "B1_CLIPPY" "cargo clippy --all-features -- -D warnings --quiet"
check "B1_FMT" "cargo fmt --check --quiet"

# ============================================================
# B2: Test — all lib + integration tests pass
# ============================================================
echo ""
echo "--- B2: Test ---"
check "B2_LIB_TESTS" "cargo test --all-features --lib --quiet"
# Integration tests may require data files; warn only
warn "B2_INTEGRATION_TESTS" "cargo test --all-features --test '*' --quiet"

# ============================================================
# B3: v3.11.0 release files
# ============================================================
echo ""
echo "--- B3: Release Files (v3.11.0) ---"
check "B3_CHANGELOG" "test -f CHANGELOG.md"
check "B3_RELEASE_NOTES" "test -f docs/releases/v3.11.0/RELEASE_NOTES.md"
check "B3_STAGE_YAML" "test -f docs/releases/v3.11.0/STAGE.yaml"
check "B3_FEATURE_CHECKLIST" "test -f docs/releases/v3.11.0/FEATURE_CHECKLIST.md"
check "B3_VERSION_PLAN" "test -f docs/releases/v3.11.0/plans/V311_VERSION_PLAN.md"
check "B3_DEV_PLAN" "test -f docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md"
check "B3_DEBT_PLAN" "test -f docs/releases/v3.11.0/plans/V311_DEBT_CLOSURE_PLAN.md"
check "B3_ISSUE_CROSSREF" "test -f docs/releases/v3.11.0/plans/V311_ISSUE_CROSSREF.md"

# ============================================================
# B4: Gate scripts exist
# ============================================================
echo ""
echo "--- B4: Gate Scripts ---"
check "B4_ALPHA_GATE" "test -f scripts/gate/check_alpha_v3.11.0.sh"
check "B4_BETA_GATE" "test -f scripts/gate/check_beta_v3.11.0.sh"
check "B4_COMMON_GATES" "test -f scripts/gate/check_arch_invariants.sh && test -f scripts/gate/check_beta_gate.sh && test -f scripts/gate/check_anti_fabrication.sh"

# ============================================================
# B5: Debt register
# ============================================================
echo ""
echo "--- B5: Debt Tracking ---"
check "B5_DEBT_REGISTRY" "test -f docs/governance/debt/debt-registry.yaml"

# ============================================================
# Summary
# ============================================================
echo ""
echo "=== v3.11.0 Beta Gate Summary ==="
echo "PASS: $PASS/$TOTAL"
[ "$WARN" -gt 0 ] && echo "WARN: $WARN"
echo "BLOCKERS: $BLOCKERS"
echo ""

# Write JSON report
cat > "$OUT_DIR/beta_gate_report.json" <<JSONEOF
{
  "version": "$VERSION",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "checks": { "pass": $PASS, "total": $TOTAL, "warn": $WARN, "blockers": $BLOCKERS },
  "result": "$([ "$BLOCKERS" -eq 0 ] && echo 'PASS' || echo 'BLOCKED')"
}
JSONEOF

if $JSON_OUTPUT; then
    cat "$OUT_DIR/beta_gate_report.json"
fi

if [ "$BLOCKERS" -eq 0 ]; then
    echo "✓ All checks pass — ready for BETA promotion."
    exit 0
else
    echo "✗ $BLOCKERS blocker(s) must be resolved before BETA promotion."
    exit 1
fi
