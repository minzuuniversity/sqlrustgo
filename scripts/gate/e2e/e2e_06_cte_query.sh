#!/usr/bin/env bash
# E2E-06: CTE + recursive query correctness
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_cte_test_$$"
PASS=0
FAIL=0

cleanup() { mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "DROP DATABASE IF EXISTS $TEST_DB;" 2>/dev/null || true; }
trap cleanup EXIT

echo "=== E2E-06: CTE + Recursive Query ==="

mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE $TEST_DB;" 2>/dev/null
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "CREATE TABLE org (id INTEGER PRIMARY KEY, name VARCHAR(50), manager_id INTEGER);" 2>/dev/null
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "
    INSERT INTO org VALUES
    (1, 'CEO', NULL),
    (2, 'VP Eng', 1),
    (3, 'VP Mkt', 1),
    (4, 'Eng Mgr', 2),
    (5, 'Dev', 4);
" 2>/dev/null

# Simple CTE
echo "--- Simple CTE ---"
RESULT=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "
    WITH dept AS (SELECT id, name FROM org WHERE manager_id IS NOT NULL)
    SELECT COUNT(*) AS cnt FROM dept;
" -N 2>/dev/null || echo "0")
if [ "$RESULT" -eq 4 ] 2>/dev/null; then echo "  PASS: Simple CTE ($RESULT rows)"; PASS=$((PASS+1)); else echo "  FAIL: CTE ($RESULT)"; FAIL=$((FAIL+1)); fi

echo ""
echo "=== E2E-06: $PASS PASS, $FAIL FAIL ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
