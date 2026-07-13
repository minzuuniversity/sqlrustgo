#!/usr/bin/env bash
# E2E-08: Migration (v3.9 schema → v3.10)
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_migrate_test_$$"
PASS=0
FAIL=0

cleanup() { mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "DROP DATABASE IF EXISTS $TEST_DB;" 2>/dev/null || true; }
trap cleanup EXIT

echo "=== E2E-08: Schema Migration ==="

# Create v3.9-compatible schema
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE $TEST_DB;" 2>/dev/null
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "
    CREATE TABLE v39_t1 (id INTEGER, name VARCHAR(100));
    CREATE TABLE v39_t2 (id INTEGER, ref_id INTEGER, amount DECIMAL(10,2));
    INSERT INTO v39_t1 VALUES (1, 'legacy'), (2, 'data');
    INSERT INTO v39_t2 VALUES (1, 1, 100.00), (2, 2, 200.00);
" 2>/dev/null

echo "--- v3.9 schema accessible on v3.10 ---"
ROWS=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "
    SELECT t1.name, t2.amount FROM v39_t1 t1 JOIN v39_t2 t2 ON t1.id = t2.ref_id;
" -N 2>/dev/null | wc -l | tr -d ' ')
if [ "$ROWS" -ge 2 ] 2>/dev/null; then echo "  PASS: v3.9 data queryable ($ROWS rows)"; PASS=$((PASS+1)); else echo "  FAIL: Migration data ($ROWS)"; FAIL=$((FAIL+1)); fi

echo ""
echo "=== E2E-08: $PASS PASS, $FAIL FAIL ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
