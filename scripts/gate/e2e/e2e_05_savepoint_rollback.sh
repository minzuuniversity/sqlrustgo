#!/usr/bin/env bash
# E2E-05: Savepoint + rollback chain
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_sp_test_$$"
PASS=0
FAIL=0

cleanup() { mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "DROP DATABASE IF EXISTS $TEST_DB;" 2>/dev/null || true; }
trap cleanup EXIT

echo "=== E2E-05: Savepoint + Rollback Chain ==="

mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE $TEST_DB;" 2>/dev/null
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "CREATE TABLE sp_data (id INTEGER PRIMARY KEY, val VARCHAR(50));" 2>/dev/null

# Chain: BEGIN → INSERT A → SAVEPOINT sp1 → INSERT B → SAVEPOINT sp2 → INSERT C → ROLLBACK TO sp1 → COMMIT
echo "--- Savepoint rollback chain ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "
    BEGIN;
    INSERT INTO sp_data VALUES (1, 'A');
    SAVEPOINT sp1;
    INSERT INTO sp_data VALUES (2, 'B');
    SAVEPOINT sp2;
    INSERT INTO sp_data VALUES (3, 'C');
    ROLLBACK TO sp1;
    COMMIT;
" 2>&1

# Should have only A (id=1). B and C rolled back.
ROW1=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT val FROM sp_data WHERE id=1;" -N 2>/dev/null || echo "")
ROW2=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT val FROM sp_data WHERE id=2;" -N 2>/dev/null || echo "")
ROW3=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT val FROM sp_data WHERE id=3;" -N 2>/dev/null || echo "")

if [ "$ROW1" = "A" ]; then echo "  PASS: id=1 (A) survives"; PASS=$((PASS+1)); else echo "  FAIL: id=1 ($ROW1)"; FAIL=$((FAIL+1)); fi
if [ -z "$ROW2" ]; then echo "  PASS: id=2 rolled back"; PASS=$((PASS+1)); else echo "  FAIL: id=2 persisted ($ROW2)"; FAIL=$((FAIL+1)); fi
if [ -z "$ROW3" ]; then echo "  PASS: id=3 rolled back"; PASS=$((PASS+1)); else echo "  FAIL: id=3 persisted ($ROW3)"; FAIL=$((FAIL+1)); fi

echo ""
echo "=== E2E-05: $PASS PASS, $FAIL FAIL ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
