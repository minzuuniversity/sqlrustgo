#!/usr/bin/env bash
# check_upgrade_guide.sh — GA-P1 Upgrade Guide Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 Upgrade Guide Gate ==="
[ -f docs/releases/v3.11.0/UPGRADE_GUIDE.md ] && echo "  [PASS] UPGRADE_GUIDE.md" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
grep -q "Breaking Changes" docs/releases/v3.11.0/UPGRADE_GUIDE.md && echo "  [PASS] Breaking Changes" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "Migration Steps" docs/releases/v3.11.0/UPGRADE_GUIDE.md && echo "  [PASS] Migration Steps" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_upgrade_guide.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
