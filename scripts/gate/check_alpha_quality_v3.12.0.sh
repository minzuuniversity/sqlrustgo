#!/usr/bin/env bash
# v3.12.0 Alpha Quality Gate.
#
# This script verifies quality and governance blockers. It is deliberately
# separate from the Alpha Entry check so an entry PASS cannot be mistaken for
# feature completion or gate quality.

set -uo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0
TOTAL=0
BLOCKERS=0
LOG_DIR="docs/releases/v3.12.0/logs"
mkdir -p "$LOG_DIR"

check() {
  local label="$1"
  local cmd="$2"
  local log="$LOG_DIR/alpha_quality_${label}_$(git rev-parse --short HEAD 2>/dev/null || echo unknown)_$(date +%Y%m%d_%H%M%S).log"
  TOTAL=$((TOTAL + 1))
  printf '  [%s] ' "$label"
  if eval "$cmd" >"$log" 2>&1; then
    hash=$(sha256sum "$log" 2>/dev/null | cut -d' ' -f1 || echo unavailable)
    echo "PASS (log=$log sha256=$hash)"
    PASS=$((PASS + 1))
  else
    hash=$(sha256sum "$log" 2>/dev/null | cut -d' ' -f1 || echo unavailable)
    echo "FAIL (log=$log sha256=$hash)"
    sed 's/^/    /' "$log" | tail -30
    BLOCKERS=$((BLOCKERS + 1))
  fi
}

echo "=== v3.12.0 Alpha Quality Gate ==="
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown) @ $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "Boundary: hard quality gates; registered exclusions are visible debt, not PASS evidence."
if [ "${ALPHA_QUALITY_FAST_TEST:-0}" = "1" ]; then
  echo "Mode: fast regression test fixture; production runs must not set ALPHA_QUALITY_FAST_TEST."
  SQLLOGICTEST_CMD="false"
  DEFERRED_CMD="bash scripts/gate/check_v312_deferred_followups.sh"
  IGNORE_COUNT_CMD="false"
  ANTI_IGNORE_CMD="false"
  P16_CMD="false"
  SQL_CORPUS_CMD="true"
  AFP_CMD="true"
else
  SQLLOGICTEST_CMD="bash scripts/gate/check_sqllogictest_v312.sh"
  DEFERRED_CMD="bash scripts/gate/check_v312_deferred_followups.sh"
  IGNORE_COUNT_CMD="bash scripts/gate/check_ignore_count.sh"
  ANTI_IGNORE_CMD="bash scripts/gate/check_anti_ignore_gate.sh"
  P16_CMD="bash scripts/gate/check_gate_test_integrity.sh"
  SQL_CORPUS_CMD="bash scripts/gate/check_sql_corpus_gate.sh"
  AFP_CMD="bash scripts/gate/check_anti_fabrication.sh"
fi
echo ""

echo "--- Q1: SQLLogicTest Quality ---"
check "Q1_SQLLOGICTEST_GATE" "$SQLLOGICTEST_CMD"
check "Q1_DEFERRED_FOLLOWUPS" "$DEFERRED_CMD"

echo ""
echo "--- Q2: Test Integrity and Ignore Governance ---"
check "Q2_P12_IGNORE_COUNT" "$IGNORE_COUNT_CMD"
check "Q2_ANTI_IGNORE_BUDGET" "$ANTI_IGNORE_CMD"
check "Q3_P16_GATE_TEST_INTEGRITY" "$P16_CMD"

echo ""
echo "--- Q4: Corpus and Truthfulness Smoke ---"
check "Q4_SQL_CORPUS_80" "$SQL_CORPUS_CMD"
check "Q4_ANTI_FABRICATION" "$AFP_CMD"

echo ""
echo "=== v3.12.0 Alpha Quality Summary ==="
echo "PASS: $PASS/$TOTAL"
echo "BLOCKERS: $BLOCKERS"

if [ "$BLOCKERS" -eq 0 ]; then
  echo "STATUS: ALPHA QUALITY PASS"
  exit 0
fi

echo "STATUS: ALPHA QUALITY BLOCKED"
exit 1
