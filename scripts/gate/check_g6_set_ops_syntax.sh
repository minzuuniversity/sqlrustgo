#!/usr/bin/env bash
# =============================================================================
# G6 — V310-06 PR2 / Issue #3723 C-2 (INTERSECT / EXCEPT / UNION ORDER BY)
# Syntax & Compilation Gate
#
# Verifies:
#   C-2a  INTERSECT / INTERSECT ALL  (SQL-92 set operation)
#   C-2b  EXCEPT  / EXCEPT ALL       (SQL-92 set operation)
#   C-2c  UNION with trailing ORDER BY / LIMIT / OFFSET  (lifted onto UNION)
#
# Run: bash scripts/gate/check_g6_set_ops_syntax.sh
# =============================================================================
set -euo pipefail

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
NC=$'\033[0m'

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

pass=0
fail=0

run_check() {
    local label="$1"; shift
    local log=$(mktemp)
    if "$@" > "$log" 2>&1; then
        echo -e "[G6] $label ... ${GREEN}PASS${NC}"
        pass=$((pass + 1))
    else
        echo -e "[G6] $label ... ${RED}FAIL${NC}"
        cat "$log"
        fail=$((fail + 1))
    fi
    rm -f "$log"
}

echo "# G6 — V310-06 PR2 Set Operations Syntax Gate"
echo "# Issue #3723 C-2: INTERSECT / EXCEPT / UNION ORDER BY"
echo ""

run_check "cargo build -p sqlrustgo-parser" \
    sh -c '! cargo build -p sqlrustgo-parser 2>&1 | grep -q "^error"'

run_check "cargo test -p sqlrustgo-parser --lib (0 failures)" \
    sh -c 'cargo test -p sqlrustgo-parser --lib 2>&1 | grep -q "test result: ok. .*0 failed"'

run_check "cargo clippy --all-features -D warnings" \
    sh -c '! cargo clippy --all-features -- -D warnings 2>&1 | grep -q "^error"'

run_check "test: intersect_returns_common_rows_test" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- intersect_returns_common_rows_test 2>&1 | grep -q "test result: ok"'

run_check "test: except_returns_left_minus_right_test" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- except_returns_left_minus_right_test 2>&1 | grep -q "test result: ok"'

run_check "test: UNION ORDER BY LIMIT lifted" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- test_set_op_union_order_by_limit_lifted 2>&1 | grep -q "test result: ok"'

run_check "test: INTERSECT ALL parse" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- test_set_op_intersect_all 2>&1 | grep -q "test result: ok"'

run_check "test: EXCEPT ALL parse" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- test_set_op_except_all 2>&1 | grep -q "test result: ok"'

run_check "test: UNION LIMIT lifted" \
    sh -c 'cargo test -p sqlrustgo-parser --lib -- test_set_op_union_limit_lifted 2>&1 | grep -q "test result: ok"'

echo ""
echo "============================================"
if [ "$fail" -eq 0 ]; then
    echo -e "${GREEN}G6 PASSED — $pass/$pass checks${NC}"
    exit 0
else
    echo -e "${RED}G6 FAILED — $pass passed, $fail failed${NC}"
    exit 1
fi
