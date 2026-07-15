#!/usr/bin/env bash
# RC E2E scenario: alter_rename
# Verify ALTER TABLE ... RENAME works on sqlrustgo's wire protocol.
#   1. Connect to server via mysql client.
#   2. CREATE TABLE t_old (id INT).
#   3. ALTER TABLE t_old RENAME TO t_new.
#   4. SHOW TABLES — must show t_new, not t_old.
#   5. INSERT into t_new — must succeed.
#   6. DROP TABLE t_new.
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_alter_rename_$$"
PASS=0
FAIL=0

cleanup() {
    mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root \
        -e "DROP DATABASE IF EXISTS $TEST_DB;" 2>/dev/null || true
}
trap cleanup EXIT

assert_pass() { PASS=$((PASS+1)); echo "  ✅ $1"; }
assert_fail() { FAIL=$((FAIL+1)); echo "  ❌ $1"; }

echo "=== E2E alter_rename ==="
echo "Server: 127.0.0.1:$SERVER_PORT"
echo "Test DB: $TEST_DB"
echo ""

echo "[1/6] Create database $TEST_DB"
if mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE $TEST_DB;" 2>/dev/null; then
    assert_pass "CREATE DATABASE"
else
    assert_fail "CREATE DATABASE"; exit 1
fi

echo "[2/6] CREATE TABLE t_old"
if mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" \
    -e "CREATE TABLE t_old (id INT PRIMARY KEY, name VARCHAR(64));" 2>/dev/null; then
    assert_pass "CREATE TABLE t_old"
else
    assert_fail "CREATE TABLE t_old"; exit 1
fi

echo "[3/6] ALTER TABLE t_old RENAME TO t_new"
if mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" \
    -e "ALTER TABLE t_old RENAME TO t_new;" 2>/dev/null; then
    assert_pass "ALTER TABLE RENAME"
else
    assert_fail "ALTER TABLE RENAME"; exit 1
fi

echo "[4/6] SHOW TABLES — must contain t_new, not t_old"
TABLES=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" \
    -N -e "SHOW TABLES;" 2>/dev/null | tr -d ' \r' | sort)
if echo "$TABLES" | grep -q '^t_new$' && ! echo "$TABLES" | grep -q '^t_old$'; then
    assert_pass "SHOW TABLES shows t_new (tables: $TABLES)"
else
    assert_fail "SHOW TABLES unexpected (tables: $TABLES)"; exit 1
fi

echo "[5/6] INSERT into t_new"
if mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" \
    -e "INSERT INTO t_new (id, name) VALUES (1, 'alice');" 2>/dev/null; then
    assert_pass "INSERT into t_new"
else
    assert_fail "INSERT into t_new"; exit 1
fi

echo "[6/6] SELECT from t_new"
COUNT=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" \
    -N -e "SELECT COUNT(*) FROM t_new;" 2>/dev/null | tr -d ' \r')
if [ "$COUNT" = "1" ]; then
    assert_pass "SELECT from t_new returned 1 row"
else
    assert_fail "SELECT returned '$COUNT' rows, expected 1"; exit 1
fi

echo ""
echo "=== E2E alter_rename: $PASS pass, $FAIL fail ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
