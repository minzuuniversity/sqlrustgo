#!/usr/bin/env bash
# RC-B1 - V312-RC-GA: BustubX-EDU B-track oracle corpus gate
# Source: docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md §5
#   "RC-B1 B-track oracle corpus | bash scripts/gate/check_bustubx_b_track_v312.sh
#    | Core B-track seed + exercises pass against SQLite oracle or issue-linked exclusion."
#
# STATUS (2026-09-04): SKELETON — Awaiting Phase 2 implementation.
# This stub intentionally exits 1 (NOT PASS) to comply with Round-24
# Anti-Fabrication-Policy-v1.0 and avoid fake PASS markers.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="$REPO_ROOT/tests/compat/bustubx_edu_b_track/manifest.yml"
BTRACK_ROOT="$REPO_ROOT/tests/compat/bustubx_edu_b_track"
EXPECTED_ARTIFACTS=(
  "$MANIFEST"
  "$BTRACK_ROOT/seed.sql"
  "$BTRACK_ROOT/exercises/week01_setup.sql"
  "$BTRACK_ROOT/oracle/week01_setup.expected"
)

echo "=== RC-B1 BustubX-EDU B-track oracle corpus (SKELETON) ==="
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
  echo "  - Author B-track seed.sql from bustubx_edu week01-week06 curriculum"
  echo "  - Generate SQLite oracle via 'sqlite3 < seed.sql' for each exercise"
  echo "  - Populate oracle/week{N}-{name}.expected from SQLite output"
  echo "  - Re-run this gate; manifest pass-rate must reach 6/6 week coverage"
  exit 1
fi

echo
echo "RESULT: artifacts present — falling through to execution phase (Phase 2)."
echo "Phase 2 will add: SQLRustGo vs SQLite row-count + SHA256 diff for each B-track SQL"
exit 1
