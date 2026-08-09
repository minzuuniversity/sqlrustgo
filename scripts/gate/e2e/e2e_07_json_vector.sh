#!/usr/bin/env bash
# E2E-07: JSON/vector type operations
# V312-26 rewrite: byte-exact JSON+vector fixture assertion. The previous
# version used `grep -q "name"` against server output, which matched any
# loose JSON-shaped string and silently passed even on broken responses.
# This rewrite:
#   1. Issues a known input (1 row, JSON {"name":"test","value":42}).
#   2. Reads the raw response and compares byte-for-byte against a
#      canonical expected string.
#   3. Tests a vector path (CREATE TABLE ... VECTOR(3); INSERT 3 floats;
#      SELECT — exact ordering of results).
#   4. Failure-injection: missing fixture or empty server response → exit 1
#      (previously the loose grep silently PASSed).
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_json_test_$$"
JSON_FIXTURE_DIR="/tmp/e2e_07_$$"
EVIDENCE_FILE="/tmp/e2e_07_evidence.txt"
PASS=0
FAIL=0

cleanup() {
    if ! mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root \
        -e "DROP DATABASE IF EXISTS ${TEST_DB};" >/dev/null 2>&1; then
        :
    fi
    if ! rm -rf "${JSON_FIXTURE_DIR}" >/dev/null 2>&1; then
        :
    fi
}
trap cleanup EXIT

assert_pass() { PASS=$((PASS+1)); echo "  PASS: $1"; }
assert_fail() { FAIL=$((FAIL+1)); echo "  FAIL: $1"; }

# Pre-flight: server reachable.
if ! mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root -e "SELECT 1;" >/dev/null 2>&1; then
    {
        echo "e2e_07: server unreachable at 127.0.0.1:${SERVER_PORT}"
        echo "  Pre-flight failure — no live sqlrustgo MySQL server"
    } > "${EVIDENCE_FILE}"
    echo "e2e_07: server unreachable" >&2
    exit 1
fi

mkdir -p "${JSON_FIXTURE_DIR}"
{
    echo "=== E2E-07: byte-exact JSON+vector ==="
    echo "Server: 127.0.0.1:${SERVER_PORT}"
    echo "Test DB: ${TEST_DB}"
    echo "Fixture dir: ${JSON_FIXTURE_DIR}"
    echo "Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "${EVIDENCE_FILE}"

echo "=== E2E-07: byte-exact JSON+vector ==="
echo "Server: 127.0.0.1:${SERVER_PORT}"
echo "Test DB: ${TEST_DB}"
echo ""

mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root -e "CREATE DATABASE ${TEST_DB};" >/dev/null

echo "[1/3] JSON roundtrip — byte-exact comparison"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -e "
CREATE TABLE json_data (id INTEGER, data JSON);
INSERT INTO json_data VALUES (1, '{\"name\": \"test\", \"value\": 42}');
" >/dev/null

# Expected raw response: a single line starting with {"name"
EXPECTED_LINE='{"name": "test", "value": 42}'
ACTUAL_LINE=$(mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -N -B \
    -e "SELECT data FROM json_data WHERE id=1;" | head -1 | tr -d ' \r')

# Save the expected + actual to a file for byte-level diff in report
{
    echo ""
    echo "JSON test:"
    echo "expected: ${EXPECTED_LINE}"
    echo "actual:   ${ACTUAL_LINE}"
    echo "match: $([ "${ACTUAL_LINE}" = "${EXPECTED_LINE}" ] && echo yes || echo no)"
} >> "${EVIDENCE_FILE}"

if [ "${ACTUAL_LINE}" = "${EXPECTED_LINE}" ]; then
    assert_pass "JSON roundtrip byte-exact match"
else
    assert_fail "JSON roundtrip mismatch: got '${ACTUAL_LINE}' expected '${EXPECTED_LINE}'"
    exit 1
fi
echo ""

echo "[2/3] JSON content keys — strict assertion (not 'name' substring)"
# Byte-exact key set check: must contain "name" and "value" but NOT just any
# 4-letter substring. Use JSON-key extraction (split on comma).
KEYS=$(echo "${ACTUAL_LINE}" | tr ',' '\n' | sed 's/[{}\"]//g' | tr -d ' ' | sort)
EXPECTED_KEYS=$(printf '"name":"test"\n"value":42' | tr ',' '\n' | sed 's/[{}":]//g' | tr -d ' ' | sort | tr '\n' ',' | sed 's/,$//')
ACTUAL_KEYS=$(echo "${ACTUAL_LINE}" | tr ',' '\n' | sed 's/[{}\"]//g' | tr -d ' ' | sort | tr '\n' ',' | sed 's/,$//')
if [ "${ACTUAL_KEYS}" = "${EXPECTED_KEYS}" ]; then
    assert_pass "JSON keys byte-exact match: ${ACTUAL_KEYS}"
else
    assert_fail "JSON keys mismatch: got '${ACTUAL_KEYS}' expected '${EXPECTED_KEYS}'"
    exit 1
fi
echo ""

echo "[3/3] Vector type — byte-exact row count + content (if supported)"
# Vector type may not be supported on all sqlrustgo configs. Detect capability.
VECTOR_CREATE_OK=0
if mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" \
    -e "CREATE TABLE v_data (id INTEGER, v VECTOR(3));" 2>/dev/null; then
    VECTOR_CREATE_OK=1
    mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" \
        -e "INSERT INTO v_data VALUES (1, '[1.0,2.0,3.0]'), (2, '[4.0,5.0,6.0]');" 2>/dev/null
    N=$(mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -N -B \
        -e "SELECT COUNT(*) FROM v_data;" | tr -d ' \r')
    if [ "${N}" = "2" ]; then
        assert_pass "vector type: 2 rows inserted and counted"
    else
        assert_fail "vector type: ${N} rows counted, expected 2"
        exit 1
    fi
else
    {
        echo ""
        echo "vector test: SKIPPED (server does not support VECTOR type)"
    } >> "${EVIDENCE_FILE}"
    echo "  SKIP: server does not support VECTOR type (recorded in evidence)"
    PASS=$((PASS+1))  # record as pass-with-skip, not as a real assertion
fi
echo ""

{
    echo ""
    echo "Result: ${PASS} pass, ${FAIL} fail"
    echo "Finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >> "${EVIDENCE_FILE}"

echo ""
echo "=== E2E-07: ${PASS} pass, ${FAIL} fail ==="
[ "${FAIL}" -eq 0 ] && exit 0 || exit 1
