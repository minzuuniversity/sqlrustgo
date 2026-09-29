#!/usr/bin/env bash
# v4.1.0 Alpha Gate -- composite wrapper.
#
# Alpha is split into:
#   1. Entry readiness: docs and tool entry points exist (check_alpha_entry_v4.10.sh).
#   2. Quality gate: hard governance gates and approved deferred work
#      (check_alpha_quality_v4.10.sh).
#
# A registered exclusion is visible debt, not PASS evidence.
#
# Modeled on scripts/gate/check_alpha_v3.12.0.sh per STAGE_CONFIG ALPHA stage.

set -uo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

ENTRY_STATUS=0
QUALITY_STATUS=0

echo "=== v4.1.0 Alpha Gate Composite ==="
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown) @ $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo ""

bash scripts/gate/check_alpha_entry_v4.10.sh
ENTRY_STATUS=$?

echo ""
bash scripts/gate/check_alpha_quality_v4.10.sh
QUALITY_STATUS=$?

echo ""
echo "=== v4.1.0 Alpha Gate Composite Summary ==="
echo "ENTRY_STATUS: $ENTRY_STATUS"
echo "QUALITY_STATUS: $QUALITY_STATUS"

if [ "$ENTRY_STATUS" -eq 0 ] && [ "$QUALITY_STATUS" -eq 0 ]; then
  echo "STATUS: ALPHA GATE PASS"
  exit 0
fi

if [ "$ENTRY_STATUS" -eq 0 ]; then
  echo "STATUS: ALPHA ENTRY PASS / ALPHA QUALITY BLOCKED"
else
  echo "STATUS: ALPHA GATE BLOCKED"
fi
exit 1