#!/usr/bin/env bash
# v3.12.0 Alpha Entry Check -- DRAFT -> ALPHA readiness.
#
# This script only verifies entry readiness: required planning documents,
# documentation consistency, and tool entry points. It must not be cited as
# evidence that v3.12.0 functional or quality gates are complete.

set -uo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0
TOTAL=0
BLOCKERS=0

check() {
  local label="$1"
  local cmd="$2"
  local log="/tmp/v312_alpha_entry_${label}_$$.log"
  TOTAL=$((TOTAL + 1))
  printf '  [%s] ' "$label"
  if eval "$cmd" >"$log" 2>&1; then
    echo "PASS"
    PASS=$((PASS + 1))
  else
    echo "FAIL"
    sed 's/^/    /' "$log" | tail -20
    BLOCKERS=$((BLOCKERS + 1))
  fi
  rm -f "$log"
}

echo "=== v3.12.0 Alpha Entry Check ==="
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown) @ $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "Boundary: entry readiness only; not a feature-completion or quality PASS."
echo ""

echo "--- E1: Required Draft/Alpha Documents ---"
check "E1_STAGE" "test -f docs/releases/v3.12.0/STAGE.yaml"
check "E1_VERSION_PLAN" "test -f docs/releases/v3.12.0/VERSION_PLAN.md"
check "E1_DEVELOPMENT_PLAN" "test -f docs/releases/v3.12.0/DEVELOPMENT_PLAN.md"
check "E1_TEST_PLAN" "test -f docs/releases/v3.12.0/TEST_PLAN.md"
check "E1_ISSUES_PLAN" "test -f docs/releases/v3.12.0/ISSUES_PLAN.md"
check "E1_ARCHITECTURE" "test -f docs/releases/v3.12.0/ARCHITECTURE.md"
check "E1_RELEASE_NOTES" "test -f docs/releases/v3.12.0/RELEASE_NOTES.md"
check "E1_DRAFT_ASSESSMENT" "test -f docs/releases/v3.12.0/DRAFT_ASSESSMENT_AND_ALPHA_GATE.md"

echo ""
echo "--- E2: Documentation Entry Gates ---"
check "E2_DOC_LINKS" "bash scripts/gate/check_docs_links.sh"
check "E2_DOC_CONSISTENCY" "bash scripts/gate/check_docs_consistency.sh"

echo ""
echo "--- E3: Tool Entry Points ---"
check "E3_SQLLOGICTEST_SCRIPT" "test -x scripts/gate/check_sqllogictest_v312.sh"
check "E3_SQLLOGICTEST_BUILD" "cargo build -p sqlrustgo_sqllogictest"
check "E3_SQLLOGICTEST_HELP" "cargo run -p sqlrustgo_sqllogictest -- --help"
check "E3_ALPHA_QUALITY_SCRIPT" "test -x scripts/gate/check_alpha_quality_v3.12.0.sh"

echo ""
echo "--- E4: Coverage Framework Entry Gates ---"
check "E4_COVERAGE_FRAMEWORK_DOC" "test -f docs/releases/v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md"
check "E4_COVERAGE_BASELINE_SCRIPT" "test -x scripts/gate/check_v312_coverage_baseline.sh"
check "E4_COVERAGE_CONFIG" "bash scripts/gate/check_v312_coverage_baseline.sh --check-config"

echo ""
echo "=== v3.12.0 Alpha Entry Summary ==="
echo "PASS: $PASS/$TOTAL"
echo "BLOCKERS: $BLOCKERS"

if [ "$BLOCKERS" -eq 0 ]; then
  echo "STATUS: ALPHA ENTRY PASS"
  exit 0
fi

echo "STATUS: ALPHA ENTRY BLOCKED"
exit 1
