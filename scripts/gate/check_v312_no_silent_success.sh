#!/usr/bin/env bash
# RC-B3 - V312-RC-GA: Semantic no-op guard gate
# Source: docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md §5
#   "RC-B3 Semantic no-op guard | bash scripts/gate/check_v312_no_silent_success.sh
#    | Accepted DDL/DML has observable postcondition;
#      unsupported SQL returns explicit error."
#
# STATUS (2026-09-04): SKELETON — Awaiting Phase 2 implementation.
# Linked issues: #4652 (CREATE PROCEDURE/FUNCTION silently accepts without storage),
#                #4709 (CHECK constraint silently accepts invalid rows — closed PR #4737),
#                #4701 (expression index accepts but errors at execution).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURES="$REPO_ROOT/tests/no_silent_success"

EXPECTED_ARTIFACTS=(
  "$FIXTURES/manifest.yml"
  "$FIXTURES/ddl_noop_probes.sql"
  "$FIXTURES/dml_noop_probes.sql"
  "$FIXTURES/expected/accepted_must_have_postcondition.md"
  "$FIXTURES/expected/rejected_must_return_error.md"
)

echo "=== RC-B3 Semantic no-op guard (SKELETON) ==="
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
  echo "  - DDL_NOOP: CREATE PROCEDURE body without storage → must return error"
  echo "  - DML_NOOP: CHECK constraint that silently accepts out-of-range rows"
  echo "  - For each probe: assert that 'accepted SQL' produces observable side-effect"
  echo "                    OR returns 'unsupported' error (never silent no-op)"
  exit 1
fi
exit 1
