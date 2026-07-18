#!/usr/bin/env bash
# check_sysbench_oltp.sh — GA-P1 Sysbench OLTP Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 Sysbench OLTP Gate ==="
[ -f scripts/sysbench/run_oltp.sh ] && echo "  [PASS] run_oltp.sh exists" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
bash -n scripts/sysbench/run_oltp.sh 2>/dev/null && echo "  [PASS] syntax" && PASS=$((PASS+1)) || { echo "  [FAIL] syntax"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_sysbench_oltp.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL] gate"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
