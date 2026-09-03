#!/usr/bin/env bash
# RC-B4 - V312-RC-GA: Type/function conformance gate
# Source: docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md §5
#   "RC-B4 Type/function conformance | bash scripts/gate/check_v312_type_function_semantics.sh
#    | Float, round, length/char_length, math/date/string core functions match oracle."
#
# STATUS (2026-09-04): SKELETON — Awaiting Phase 2 implementation.
# Linked issues: #4674 (CHAR_LENGTH wrong), #4676 (MOD/POWER/LOG/EXP/SQRT unimplemented),
#                #4675 (POSITION/LOCATE unimplemented), #4670 (CEIL/FLOOR/TRUNC partial),
#                #4721 (round(real,int) regression — closed), #4698 (GREATEST/LEAST unimplemented).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURES="$REPO_ROOT/tests/type_function_semantics"

EXPECTED_ARTIFACTS=(
  "$FIXTURES/manifest.yml"
  "$FIXTURES/probe_numeric.sql"
  "$FIXTURES/probe_string.sql"
  "$FIXTURES/probe_date.sql"
  "$FIXTURES/oracle/probe_numeric.expected"
  "$FIXTURES/oracle/probe_string.expected"
  "$FIXTURES/oracle/probe_date.expected"
)

echo "=== RC-B4 Type/function conformance (SKELETON) ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Repo:      $REPO_ROOT"
echo "Branch:    $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')"
echo "HEAD:      $(git rev-parse --short HEAD 2>/dev/null || echo '?')"
echo

MISSING=0
for f in "${EXPECTED_ARTIFACTS[@]}"; do
  if [ ! -f "$f" ]; then
    echo "  MISSING: $f"
    MISSING=$((MISSING+1))
  fi
done

if [ "$MISSING" -gt 0 ]; then
  echo
  echo "RESULT: STUB — $MISSING required fixture(s) missing."
  echo "ACTION REQUIRED (Phase 2):"
  echo "  - Probe numeric: ROUND/FLOOR/CEIL/MOD/POWER/LOG/EXP/SQRT/GREATEST/LEAST"
  echo "  - Probe string:  CHAR_LENGTH/CHARACTER_LENGTH/POSITION/LOCATE/HEX/MD5/SHA"
  echo "  - Probe date:    DATE_TRUNC/TIMESTAMPDIFF/strftime"
  echo "  - Capture SQLite oracle row-by-row via .mode line"
  echo "  - Diff against SQLRustGo output (cmp or sha256 per row)"
  echo "  - Bypass cases must issue-link GA-blocker #4674 OR claim-caveat #4676/#4675/#4670/#4698"
  exit 1
fi
exit 1
