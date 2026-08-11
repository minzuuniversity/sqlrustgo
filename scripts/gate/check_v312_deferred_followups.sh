#!/usr/bin/env bash
# v3.12 deferred follow-up visibility and approval check.
#
# Registered exclusions are not PASS evidence. This script only verifies that
# each deferred SQLLogicTest exclusion is bound to a concrete Gitea issue with
# owner, expiry, non-blocking disposition, and close boundary.

set -uo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

EXCLUSIONS="docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml"
FAIL=0

echo "=== v3.12 Deferred Follow-up Check ==="
echo "exclusions: $EXCLUSIONS"

if [ ! -f "$EXCLUSIONS" ]; then
  echo "FAIL: exclusions registry missing"
  exit 1
fi

if ! grep -q "^status: active" "$EXCLUSIONS"; then
  echo "FAIL: exclusions registry is not active"
  exit 1
fi

ITEMS=$(grep -c "^  - id:" "$EXCLUSIONS" 2>/dev/null || echo 0)
GITEA_REFS=0
OPENSPEC_ONLY=0
MISSING_OWNER=0
MISSING_EXPIRY=0
MISSING_BOUNDARY=0
BLOCKING_TRUE=0
INVALID_REF=0

while IFS= read -r line; do
  item_id=$(echo "$line" | sed 's/^  - id: //')
  block=$(grep -A 14 "^  - id: ${item_id}" "$EXCLUSIONS" 2>/dev/null || echo "")
  ref=$(echo "$block" | grep "follow_up_issue" | head -1 | sed 's/.*follow_up_issue[^:]*:[[:space:]]*//' | tr -d '"' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

  if echo "$ref" | grep -qE "^#[0-9]+"; then
    GITEA_REFS=$((GITEA_REFS + 1))
  elif echo "$ref" | grep -q "openspec/"; then
    OPENSPEC_ONLY=$((OPENSPEC_ONLY + 1))
  else
    INVALID_REF=$((INVALID_REF + 1))
  fi

  echo "$block" | grep -q "^[[:space:]]*owner:" || MISSING_OWNER=$((MISSING_OWNER + 1))
  echo "$block" | grep -qE "^[[:space:]]*(expiry|v3.13_expiry):" || MISSING_EXPIRY=$((MISSING_EXPIRY + 1))
  echo "$block" | grep -q "^[[:space:]]*close_boundary:" || MISSING_BOUNDARY=$((MISSING_BOUNDARY + 1))
  if echo "$block" | grep -q "^[[:space:]]*v3.12_blocking:[[:space:]]*true"; then
    BLOCKING_TRUE=$((BLOCKING_TRUE + 1))
  fi
done < <(grep "^  - id:" "$EXCLUSIONS" 2>/dev/null)

echo "Exclusion items: $ITEMS"
echo "Gitea issue references: $GITEA_REFS"
echo "OpenSpec-only references: $OPENSPEC_ONLY"
echo "Invalid or missing references: $INVALID_REF"
echo "Missing owner: $MISSING_OWNER"
echo "Missing expiry: $MISSING_EXPIRY"
echo "Missing close boundary: $MISSING_BOUNDARY"
echo "v3.12_blocking=true items: $BLOCKING_TRUE"

if [ "$ITEMS" -eq 0 ]; then
  echo "FAIL: no exclusion items found"
  FAIL=$((FAIL + 1))
fi
if [ "$GITEA_REFS" -ne "$ITEMS" ]; then
  echo "FAIL: every deferred exclusion must bind to a Gitea issue (#NNNN)"
  FAIL=$((FAIL + 1))
fi
if [ "$OPENSPEC_ONLY" -ne 0 ]; then
  echo "FAIL: OpenSpec-only references are not sufficient for Alpha/GA governance"
  FAIL=$((FAIL + 1))
fi
if [ "$INVALID_REF" -ne 0 ] || [ "$MISSING_OWNER" -ne 0 ] || [ "$MISSING_EXPIRY" -ne 0 ] || [ "$MISSING_BOUNDARY" -ne 0 ]; then
  echo "FAIL: deferred items are missing required owner/expiry/close-boundary metadata"
  FAIL=$((FAIL + 1))
fi
if [ "$BLOCKING_TRUE" -ne 0 ]; then
  echo "FAIL: v3.12-blocking deferred items cannot pass Alpha quality"
  FAIL=$((FAIL + 1))
fi

if [ "$FAIL" -eq 0 ]; then
  echo "STATUS: DEFERRED FOLLOW-UP VISIBILITY PASS"
  exit 0
fi

echo "STATUS: DEFERRED FOLLOW-UP VISIBILITY BLOCKED"
exit 1
