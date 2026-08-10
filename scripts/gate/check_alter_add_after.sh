#!/usr/bin/env bash
# check_alter_add_after.sh — GA-P1 ALTER ADD COLUMN AFTER/FIRST Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 ALTER ADD COLUMN AFTER/FIRST Gate ==="
[ -f docs/ALTER_ADD_COLUMN_AFTER.md ] && echo "  [PASS] ALTER_ADD_COLUMN_AFTER.md" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
grep -q "AFTER" docs/ALTER_ADD_COLUMN_AFTER.md && echo "  [PASS] AFTER documented" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "FIRST" docs/ALTER_ADD_COLUMN_AFTER.md && echo "  [PASS] FIRST documented" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_alter_add_after.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
