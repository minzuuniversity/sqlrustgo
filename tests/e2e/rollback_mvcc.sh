#!/usr/bin/env bash
# RC E2E scenario: rollback_mvcc
# Verify MVCC transaction commit/rollback across two sessions
# (session A: write + commit, session B: visibility check).
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_tx_test_$$"
PASS=0
FAIL=0

cleanup() { mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "DROP DATABASE IF EXISTS $TEST_DB;" 2>/dev/null || true; }
trap cleanup EXIT

echo "=== E2E-02: Transaction Commit + Rollback ==="

# Setup
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE $TEST_DB;" 2>/dev/null
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "CREATE TABLE txn_test (id INTEGER PRIMARY KEY, val INTEGER);" 2>/dev/null

# Test 1: COMMIT persists data
echo "--- Test 1: COMMIT persists data ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "BEGIN; INSERT INTO txn_test VALUES (1, 100); COMMIT;" 2>&1
ROWS=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT COUNT(*) AS cnt FROM txn_test;" -N 2>&1 || echo "0")
if [ "$ROWS" -ge 1 ] 2>/dev/null; then echo "  PASS: COMMIT persists"; PASS=$((PASS+1)); else echo "  FAIL: COMMIT ($ROWS)"; FAIL=$((FAIL+1)); fi

# Test 2: ROLLBACK discards data
echo "--- Test 2: ROLLBACK discards data ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "BEGIN; INSERT INTO txn_test VALUES (2, 200); ROLLBACK;" 2>&1
ROWS=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT COUNT(*) AS cnt FROM txn_test WHERE id=2;" -N 2>&1 || echo "0")
if [ "$ROWS" -eq 0 ] 2>/dev/null; then echo "  PASS: ROLLBACK discards"; PASS=$((PASS+1)); else echo "  FAIL: ROLLBACK ($ROWS)"; FAIL=$((FAIL+1)); fi

# Test 3: Nested transaction (SAVEPOINT)
echo "--- Test 3: SAVEPOINT ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "BEGIN; INSERT INTO txn_test VALUES (3, 300); SAVEPOINT sp1; INSERT INTO txn_test VALUES (4, 400); ROLLBACK TO sp1; COMMIT;" 2>&1
ROWS=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT COUNT(*) AS cnt FROM txn_test WHERE id=3;" -N 2>&1 || echo "0")
ROWS2=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT COUNT(*) AS cnt FROM txn_test WHERE id=4;" -N 2>&1 || echo "0")
if [ "$ROWS" -ge 1 ] && [ "$ROWS2" -eq 0 ] 2>/dev/null; then
    echo "  PASS: SAVEPOINT (id=3: $ROWS, id=4: $ROWS2)"; PASS=$((PASS+1))
else
    echo "  FAIL: SAVEPOINT (id=3: $ROWS, id=4: $ROWS2)"; FAIL=$((FAIL+1))
fi

echo ""
echo "=== E2E-02: $PASS PASS, $FAIL FAIL ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
