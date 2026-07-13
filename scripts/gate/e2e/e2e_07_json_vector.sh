#!/usr/bin/env bash
# E2E-07: JSON/vector type operations
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_json_test_$$"
PASS=0
FAIL=0

cleanup() { mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "DROP DATABASE IF EXISTS $TEST_DB;" 2>/dev/null || true; }
trap cleanup EXIT

echo "=== E2E-07: JSON Type Operations ==="

mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE $TEST_DB;" 2>/dev/null
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "CREATE TABLE json_data (id INTEGER, data JSON);" 2>/dev/null

echo "--- JSON INSERT and SELECT ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "INSERT INTO json_data VALUES (1, '{\"name\": \"test\", \"value\": 42}');" 2>/dev/null
RESULT=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D "$TEST_DB" -e "SELECT data FROM json_data WHERE id=1;" -N 2>/dev/null || echo "")
echo "  Got: $RESULT"
if echo "$RESULT" | grep -q "name"; then echo "  PASS: JSON data roundtrip"; PASS=$((PASS+1)); else echo "  FAIL: JSON roundtrip ($RESULT)"; FAIL=$((FAIL+1)); fi

echo ""
echo "=== E2E-07: $PASS PASS, $FAIL FAIL ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
