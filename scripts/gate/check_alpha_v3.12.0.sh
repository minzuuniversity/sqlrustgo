#!/usr/bin/env bash
# v3.12.0 Alpha Gate — DRAFT -> ALPHA promotion entry.
#
# This script validates that v3.12.0 is ready for active implementation.
# It does not claim feature completion.

set -uo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

PASS=0
TOTAL=0
BLOCKERS=0

check() {
  local label="$1"
  local cmd="$2"
  TOTAL=$((TOTAL + 1))
  printf '  [%s] ' "$label"
  if eval "$cmd" >/tmp/v312_alpha_gate_$$.log 2>&1; then
    echo "PASS"
    PASS=$((PASS + 1))
  else
    echo "FAIL"
    BLOCKERS=$((BLOCKERS + 1))
  fi
}

echo "=== v3.12.0 Alpha Gate ==="
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown) @ $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo ""

echo "--- A1: Required Draft/Alpha Documents ---"
check "A1_STAGE" "test -f docs/releases/v3.12.0/STAGE.yaml"
check "A1_VERSION_PLAN" "test -f docs/releases/v3.12.0/VERSION_PLAN.md"
check "A1_DEVELOPMENT_PLAN" "test -f docs/releases/v3.12.0/DEVELOPMENT_PLAN.md"
check "A1_TEST_PLAN" "test -f docs/releases/v3.12.0/TEST_PLAN.md"
check "A1_ISSUES_PLAN" "test -f docs/releases/v3.12.0/ISSUES_PLAN.md"
check "A1_ARCHITECTURE" "test -f docs/releases/v3.12.0/ARCHITECTURE.md"
check "A1_RELEASE_NOTES" "test -f docs/releases/v3.12.0/RELEASE_NOTES.md"
check "A1_DRAFT_ASSESSMENT" "test -f docs/releases/v3.12.0/DRAFT_ASSESSMENT_AND_ALPHA_GATE.md"

echo ""
echo "--- A2: Documentation Gates ---"
check "A2_DOC_LINKS" "bash scripts/gate/check_docs_links.sh"
check "A2_DOC_CONSISTENCY" "bash scripts/gate/check_docs_consistency.sh"

echo ""
echo "--- A3: SQLLogicTest Gate Entry ---"
check "A3_SQLLOGICTEST_SCRIPT" "test -x scripts/gate/check_sqllogictest_v312.sh"
check "A3_SQLLOGICTEST_ENTRY" "bash scripts/gate/check_sqllogictest_v312.sh"

echo ""
echo "--- A4: Branch Sanity ---"
check "A4_BRANCH" "git rev-parse --abbrev-ref HEAD | grep -q '^develop/v3.12.0$'"

echo ""
echo "=== v3.12.0 Alpha Gate Summary ==="
echo "PASS: $PASS/$TOTAL"
echo "BLOCKERS: $BLOCKERS"

rm -f /tmp/v312_alpha_gate_$$.log

if [ "$BLOCKERS" -eq 0 ]; then
  echo "STATUS: ALPHA GATE ENTRY PASS"
  exit 0
fi

echo "STATUS: ALPHA GATE BLOCKED"
exit 1
