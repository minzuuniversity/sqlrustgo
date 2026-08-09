#!/usr/bin/env bash
# check_regression_ci.sh — GA-P1 Regression Test Suite Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 Regression Test Suite Gate ==="
[ -f docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md ] && echo "  [PASS] REGRESSION_TEST_SUITE.md" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
grep -q "TPC-H" docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md && echo "  [PASS] TPC-H documented" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "Chaos Soak" docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md && echo "  [PASS] Chaos Soak" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "Upgrade" docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md && echo "  [PASS] Upgrade documented" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "Coverage" docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md && echo "  [PASS] Coverage documented" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_regression_ci.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
