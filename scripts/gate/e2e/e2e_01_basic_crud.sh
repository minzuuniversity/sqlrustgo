#!/usr/bin/env bash
# E2E-01: Basic CRUD via MySQL wire protocol
# Connect to sqlrustgo via mysql client, create table, insert/select/update/delete
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_crud_test_$$"
PASS=0
FAIL=0

cleanup() {
    echo "Cleanup: dropping test database..."
    mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "DROP DATABASE IF EXISTS $TEST_DB;" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== E2E-01: Basic CRUD ==="

# 1. Create database
echo "--- Step 1: Create database ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE $TEST_DB;" 2>&1
if [ $? -eq 0 ]; then echo "  PASS: CREATE DATABASE"; PASS=$((PASS+1)); else echo "  FAIL: CREATE DATABASE"; FAIL=$((FAIL+1)); fi

# 2. Use database and create table
echo "--- Step 2: CREATE TABLE ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "
    CREATE TABLE users (
        id INTEGER PRIMARY KEY,
        name VARCHAR(100),
        email VARCHAR(200)
    );
" 2>&1
if [ $? -eq 0 ]; then echo "  PASS: CREATE TABLE"; PASS=$((PASS+1)); else echo "  FAIL: CREATE TABLE"; FAIL=$((FAIL+1)); fi

# 3. INSERT
echo "--- Step 3: INSERT ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "
    INSERT INTO users (id, name, email) VALUES
    (1, 'Alice', 'alice@example.com'),
    (2, 'Bob', 'bob@example.com'),
    (3, 'Charlie', 'charlie@example.com');
" 2>&1
if [ $? -eq 0 ]; then echo "  PASS: INSERT"; PASS=$((PASS+1)); else echo "  FAIL: INSERT"; FAIL=$((FAIL+1)); fi

# 4. SELECT
echo "--- Step 4: SELECT ---"
ROWS=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT COUNT(*) AS cnt FROM users;" -N 2>&1 || echo "0")
if [ "$ROWS" = "3" ]; then echo "  PASS: SELECT ($ROWS rows)"; PASS=$((PASS+1)); else echo "  FAIL: SELECT (got $ROWS, expected 3)"; FAIL=$((FAIL+1)); fi

# 5. UPDATE
echo "--- Step 5: UPDATE ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "UPDATE users SET name='Alice Updated' WHERE id=1;" 2>&1
NAME=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT name FROM users WHERE id=1;" -N 2>&1 || echo "")
if [ "$NAME" = "Alice Updated" ]; then echo "  PASS: UPDATE"; PASS=$((PASS+1)); else echo "  FAIL: UPDATE (got $NAME)"; FAIL=$((FAIL+1)); fi

# 6. DELETE
echo "--- Step 6: DELETE ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "DELETE FROM users WHERE id=3;" 2>&1
ROWS=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT COUNT(*) AS cnt FROM users;" -N 2>&1 || echo "0")
if [ "$ROWS" = "2" ]; then echo "  PASS: DELETE ($ROWS rows remain)"; PASS=$((PASS+1)); else echo "  FAIL: DELETE (got $ROWS, expected 2)"; FAIL=$((FAIL+1)); fi

# 7. DROP TABLE
echo "--- Step 7: DROP TABLE ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "DROP TABLE users;" 2>&1
if [ $? -eq 0 ]; then echo "  PASS: DROP TABLE"; PASS=$((PASS+1)); else echo "  FAIL: DROP TABLE"; FAIL=$((FAIL+1)); fi

echo ""
echo "=== E2E-01: $PASS PASS, $FAIL FAIL ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
