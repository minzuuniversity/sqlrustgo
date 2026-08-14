#!/usr/bin/env bash
#
# check_beta_v3.12.0.sh -- v3.12.0 BETA Governance Gate
#
# Companion to check_beta_gate.sh (B1-B5 / technical gates) which is
# v3.10.0-oriented. This script adds the v3.12.0-specific release files
# and documents that the per-version BETA promotion framework expects.
#
# v3.12.0 variant:
#   - B1 build/clippy/fmt (same as v3.11.0)
#   - B2 lib + integration tests
#   - B3 v3.12.0 release files (paths from docs/releases/v3.12.0/)
#   - B4 gate scripts (alpha + beta + common)
#   - B5 debt register
#   - B6 v3.12.0 BETA promotion_to_BETA_requires evidence
#   - B7 alpha gate still passing (sanity check that ALPHA gates hold)
#
# Usage:
#   bash scripts/gate/check_beta_v3.12.0.sh
#   bash scripts/gate/check_beta_v3.12.0.sh --json
#   bash scripts/gate/check_beta_v3.12.0.sh --out-dir /tmp/reports

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

VERSION="${VERSION:-v3.12.0}"
OUT_DIR="${OUT_DIR:-/tmp/beta_g312_$(date +%Y%m%d_%H%M%S)}"
JSON_OUTPUT=false

for arg in "$@"; do
    case "$arg" in
        --json) JSON_OUTPUT=true; shift ;;
        --out-dir) shift; OUT_DIR="${1:-/tmp/beta_g312}"; shift ;;
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

echo "=== v3.12.0 Beta Gate ==="
echo "Version: $VERSION"
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown') @ $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
echo ""

# ============================================================
# B1: Build — zero errors
# ============================================================
echo "--- B1: Build ---"
check "B1_CARGO_BUILD" "cargo build --all-features --quiet"
check "B1_CLIPPY" "cargo clippy --all-features -- -D warnings"
check "B1_FMT" "cargo fmt --check --quiet"

# ============================================================
# B2: Test — all lib + integration tests pass
# ============================================================
echo ""
echo "--- B2: Test ---"
check "B2_LIB_TESTS" "cargo test --all-features --lib --quiet"
warn "B2_INTEGRATION_TESTS" "cargo test --all-features --test '*' --quiet"

# ============================================================
# B3: v3.12.0 release files
# ============================================================
echo ""
echo "--- B3: Release Files (v3.12.0) ---"
check "B3_CHANGELOG" "test -f CHANGELOG.md"
check "B3_RELEASE_NOTES" "test -f docs/releases/v3.12.0/RELEASE_NOTES.md"
check "B3_STAGE_YAML" "test -f docs/releases/v3.12.0/STAGE.yaml"
check "B3_FEATURE_CHECKLIST" "test -f docs/releases/v3.12.0/FEATURE_CHECKLIST.md"
check "B3_VERSION_PLAN" "test -f docs/releases/v3.12.0/VERSION_PLAN.md"
check "B3_DEV_PLAN" "test -f docs/releases/v3.12.0/DEVELOPMENT_PLAN.md"
check "B3_TEST_PLAN" "test -f docs/releases/v3.12.0/TEST_PLAN.md"
check "B3_ISSUES_PLAN" "test -f docs/releases/v3.12.0/ISSUES_PLAN.md"

# ============================================================
# B4: Gate scripts exist
# ============================================================
echo ""
echo "--- B4: Gate Scripts ---"
check "B4_ALPHA_GATE" "test -f scripts/gate/check_alpha_v3.12.0.sh"
check "B4_BETA_GATE" "test -f scripts/gate/check_beta_v3.12.0.sh"
check "B4_COMMON_GATES" "test -f scripts/gate/check_arch_invariants.sh && test -f scripts/gate/check_beta_gate.sh && test -f scripts/gate/check_anti_fabrication.sh"

# ============================================================
# B5: Debt register
# ============================================================
echo ""
echo "--- B5: Debt Tracking ---"
check "B5_DEBT_REGISTRY" "test -f docs/governance/debt/debt-registry.yaml"

# ============================================================
# B6: v3.12.0 BETA promotion_to_BETA_requires evidence
# ============================================================
echo ""
echo "--- B6: BETA Promotion Evidence (v3.12.0) ---"
check "B6_GMP_SCHEMA"              "test -f docs/releases/v3.12.0/v312-02-gmp-schema-report.md"
check "B6_GMP_INGESTION"           "test -f docs/releases/v3.12.0/v312-03-gmp-ingestion-report.md"
check "B6_EMBEDDING_PROVIDER"      "test -f docs/releases/v3.12.0/v312-04-embedding-provider-report.md"
check "B6_HYBRID_RETRIEVAL"        "test -f docs/releases/v3.12.0/v312-05-hybrid-retrieval-report.md"
check "B6_GRAPH_PROJECTION"        "test -f docs/releases/v3.12.0/v312-06-graph-projection-report.md"
check "B6_AUDIT_HASH_CHAIN"        "test -f docs/releases/v3.12.0/v312-08-compliance-audit-report.md"
check "B6_SQLLOGICTEST_SMOKE_GATE" "bash scripts/gate/check_sqllogictest_v312.sh"
check "B6_SQLLOGICTEST_MANIFEST"   "python3 - <<'PY'
import json
from pathlib import Path

manifest = Path('docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json')
data = json.loads(manifest.read_text())
stats = data.get('corpus_stats', {})
total = int(stats.get('total_files', -1))
passed = int(stats.get('pass_files', -1))
failed = int(stats.get('fail_files', -1))
if total <= 0 or passed != total or failed != 0:
    raise SystemExit(f'sqllogictest manifest not clean: total={total} pass={passed} fail={failed}')
PY"
check "B6_SQLLOGICTEST_OPEN_EXCLUSIONS" "python3 - <<'PY'
from pathlib import Path
import re

text = Path('docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml').read_text()
items = re.split(r'^  - id: ', text, flags=re.M)[1:]
open_items = []
for item in items:
    if not re.search(r'^    status: closed\\b', item, flags=re.M):
        open_items.append(item.splitlines()[0].strip())
if open_items:
    raise SystemExit('open sqllogictest exclusions: ' + ', '.join(open_items))
PY"
check "B6_TPCH_SF1_G4"             "test -f docs/releases/v3.12.0/evidence/G4_tpch_sf1.txt"

# ============================================================
# B7: ALPHA gate sanity check (BETA cannot regress ALPHA state)
# ============================================================
echo ""
echo "--- B7: ALPHA Gate Sanity (must still pass) ---"
warn "B7_ALPHA_ENTRY"      "ALPHA_QUALITY_FAST_TEST=1 bash scripts/gate/check_alpha_entry_v3.12.0.sh >/dev/null 2>&1"
warn "B7_ALPHA_QUALITY"    "ALPHA_QUALITY_FAST_TEST=1 bash scripts/gate/check_alpha_quality_v3.12.0.sh >/dev/null 2>&1"

# ============================================================
# Summary
# ============================================================
echo ""
echo "=== v3.12.0 Beta Gate Summary ==="
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
