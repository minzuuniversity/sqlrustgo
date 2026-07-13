#!/usr/bin/env bash
# RC E2E scenario: union_set_ops
# Verify set operations (UNION, UNION ALL, INTERSECT, EXCEPT) work on sqlrustgo.
#   1. Create test database with two tables having overlapping rows.
#   2. SELECT ... UNION — distinct rows.
#   3. SELECT ... UNION ALL — duplicate rows preserved.
#   4. SELECT ... INTERSECT — common rows.
#   5. SELECT ... EXCEPT — left-only rows.
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_setops_$$"
PASS=0
FAIL=0

cleanup() {
    mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root \
        -e "DROP DATABASE IF EXISTS $TEST_DB;" 2>/dev/null || true
}
trap cleanup EXIT

assert_pass() { PASS=$((PASS+1)); echo "  ✅ $1"; }
assert_fail() { FAIL=$((FAIL+1)); echo "  $1"; }

assert_count() {
    local label="$1" expected="$2" actual="$3"
    if [ "$actual" = "$expected" ]; then
        assert_pass "$label: got $actual rows (expected $expected)"
    else
        assert_fail "❌ $label: got $actual rows, expected $expected"
        FAIL=$((FAIL+0))  # already incremented
    fi
}

echo "=== E2E union_set_ops ==="
echo "Server: 127.0.0.1:$SERVER_PORT"
echo "Test DB: $TEST_DB"
echo ""

echo "[setup] Create database and tables a, b"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE $TEST_DB;" 2>/dev/null
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" -e "
CREATE TABLE a (id INT);
CREATE TABLE b (id INT);
INSERT INTO a VALUES (1), (2), (3);
INSERT INTO b VALUES (2), (3), (4);
" 2>/dev/null
assert_pass "Setup: a={1,2,3}, b={2,3,4}"
echo ""

echo "[1/4] UNION (distinct)"
N=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" -N -B \
    -e "SELECT id FROM a UNION SELECT id FROM b;" 2>/dev/null | wc -l | tr -d ' ')
assert_count "UNION" "4" "$N"
echo ""

echo "[2/4] UNION ALL (with duplicates)"
N=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" -N -B \
    -e "SELECT id FROM a UNION ALL SELECT id FROM b;" 2>/dev/null | wc -l | tr -d ' ')
assert_count "UNION ALL" "6" "$N"
echo ""

echo "[3/4] INTERSECT"
N=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" -N -B \
    -e "SELECT id FROM a INTERSECT SELECT id FROM b;" 2>/dev/null | wc -l | tr -d ' ')
assert_count "INTERSECT" "2" "$N"
echo ""

echo "[4/4] EXCEPT"
N=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" -N -B \
    -e "SELECT id FROM a EXCEPT SELECT id FROM b;" 2>/dev/null | wc -l | tr -d ' ')
assert_count "EXCEPT" "1" "$N"
echo ""

echo "=== E2E union_set_ops: $PASS pass, $FAIL fail ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
