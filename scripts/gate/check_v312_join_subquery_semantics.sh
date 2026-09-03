#!/usr/bin/env bash
# RC-B5 - V312-RC-GA: Join/subquery correctness gate
# Source: docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md §5
#   "RC-B5 Join/subquery correctness | bash scripts/gate/check_v312_join_subquery_semantics.sh
#    | JOIN USING/NATURAL JOIN, scalar subquery, ANY/ALL, HAVING multi-condition match oracle."
#
# STATUS (2026-09-04): SKELETON — Awaiting Phase 2 implementation.
# Linked issues: #4668 (NATURAL JOIN multi-col → Cartesian product — GA-blocker),
#                #4649 (LEFT JOIN USING → Cartesian product — closed via PR #4732),
#                #4636 (correlated scalar subquery — closed via PR #4731),
#                #4656 (> ALL / = ANY empty result — closed via PR #4737).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURES="$REPO_ROOT/tests/join_subquery_semantics"

EXPECTED_ARTIFACTS=(
  "$FIXTURES/manifest.yml"
  "$FIXTURES/join_using.sql"
  "$FIXTURES/natural_join.sql"
  "$FIXTURES/scalar_subquery.sql"
  "$FIXTURES/any_all_subquery.sql"
  "$FIXTURES/having_multi.sql"
  "$FIXTURES/oracle/{join_using,natural_join,scalar_subquery,any_all_subquery,having_multi}.expected"
)

echo "=== RC-B5 Join/subquery correctness (SKELETON) ==="
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
  echo "  - join_using.sql: LEFT/RIGHT/INNER JOIN USING(col) + USING(col1,col2) multi-col"
  echo "  - natural_join.sql: NATURAL JOIN on common-named column(s)"
  echo "  - scalar_subquery.sql: WHERE x = (SELECT ... FROM b WHERE b.id = a.id)"
  echo "  - any_all_subquery.sql: > ALL(...), = ANY(...), IN (SELECT ...) variants"
  echo "  - having_multi.sql: HAVING + multiple AND/OR conditions"
  echo "  - oracle/*.expected: SQLite-generated goldens (per-row SHA256 comparison)"
  echo "  - GA-blocker bypass must issue-link #4668 until fixed"
  exit 1
fi
exit 1
