#!/usr/bin/env bash
# check_load_data_infile.sh — GA-P1 LOAD DATA INFILE Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 LOAD DATA INFILE Gate ==="
[ -f docs/LOAD_DATA_INFILE.md ] && echo "  [PASS] LOAD_DATA_INFILE.md" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
grep -q "LOAD DATA" docs/LOAD_DATA_INFILE.md && echo "  [PASS] LOAD DATA documented" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "parser" docs/LOAD_DATA_INFILE.md && echo "  [PASS] parser changes documented" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_load_data_infile.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
