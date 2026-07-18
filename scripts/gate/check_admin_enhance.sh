#!/usr/bin/env bash
# check_admin_enhance.sh — GA-P1 Admin Enhancement Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 Admin Enhancement Gate ==="
[ -f scripts/admin/admin-status.sh ] && echo "  [PASS] admin-status.sh" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
[ -f scripts/admin/admin-flush-logs.sh ] && echo "  [PASS] admin-flush-logs.sh" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
bash -n scripts/admin/admin-status.sh 2>/dev/null && echo "  [PASS] status syntax" && PASS=$((PASS+1)) || { echo "  [FAIL] syntax"; FAIL=$((FAIL+1)); }
bash -n scripts/admin/admin-flush-logs.sh 2>/dev/null && echo "  [PASS] flush syntax" && PASS=$((PASS+1)) || { echo "  [FAIL] syntax"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_admin_enhance.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL] gate"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
