#!/usr/bin/env bash
# RC E2E scenario: sysbench_wired
# Run a sysbench OLTP-read-only workload against sqlrustgo's MySQL wire
# protocol to validate end-to-end throughput.
#
# V312-26 rewrite: explicit `sysbench --version` pre-flight. If sysbench is
# not installed, the script exits 1 with a clear stderr message — the
# previous version silently fell back to a 200-statement OLTP read loop,
# which was a WARN-only anti-pattern.
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_sysbench_$$"
SYSBENCH_OUT="/tmp/sysbench_$$.out"
EVIDENCE_FILE="/tmp/sysbench_wired_evidence.txt"
PASS=0
FAIL=0

cleanup() {
    if ! mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root \
        -e "DROP DATABASE IF EXISTS ${TEST_DB};" >/dev/null 2>&1; then
        :
    fi
    if ! rm -f "${SYSBENCH_OUT}" >/dev/null 2>&1; then
        :
    fi
}
trap cleanup EXIT

assert_pass() { PASS=$((PASS+1)); echo "  PASS: $1"; }
assert_fail() { FAIL=$((FAIL+1)); echo "  FAIL: $1"; }

# Pre-flight: sysbench must be installed. No silent fallback. (V312-26.)
if ! command -v sysbench >/dev/null 2>&1; then
    {
        echo "sysbench_wired: sysbench not found"
        echo "  Pre-flight failure — sysbench binary required for OLTP-read workload"
        echo "  Server: 127.0.0.1:${SERVER_PORT}"
        echo "  Test DB: ${TEST_DB}"
        echo "  sysbench --version: not found in PATH"
    } > "${EVIDENCE_FILE}"
    echo "sysbench not found in PATH" >&2
    echo "Pre-flight failure: install sysbench (e.g. apt-get install sysbench) and retry" >&2
    exit 1
fi

SYSBENCH_VERSION=$(sysbench --version 2>&1 || echo "unknown")
{
    echo "=== E2E sysbench_wired: real sysbench oltp_read_only ==="
    echo "Server: 127.0.0.1:${SERVER_PORT}"
    echo "Test DB: ${TEST_DB}"
    echo "sysbench version: ${SYSBENCH_VERSION}"
    echo "Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "${EVIDENCE_FILE}"

echo "=== E2E sysbench_wired: real sysbench oltp_read_only ==="
echo "Server: 127.0.0.1:${SERVER_PORT}"
echo "Test DB: ${TEST_DB}"
echo "sysbench: ${SYSBENCH_VERSION}"
echo ""

echo "[1/4] Prepare: create sbtest1 with 100 rows"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root -e "CREATE DATABASE ${TEST_DB};"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -e "
CREATE TABLE sbtest1 (
    id INT PRIMARY KEY,
    k INT NOT NULL,
    c VARCHAR(64) NOT NULL,
    pad VARCHAR(64) NOT NULL
);
"
for i in $(seq 1 100); do
    mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" \
        -e "INSERT INTO sbtest1 VALUES ($i, $((RANDOM % 1000)), 'payload-$i', 'pad-$i');" >/dev/null
done
N=$(mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -N -B \
    -e "SELECT COUNT(*) FROM sbtest1;" | tr -d ' \r')
if [ "${N}" = "100" ]; then
    assert_pass "sbtest1 prepared with 100 rows"
else
    assert_fail "sbtest1 has ${N} rows, expected 100"
    exit 1
fi
echo ""

echo "[2/4] sysbench oltp_read_only (10s, 4 threads)"
# Real sysbench run; no `|| true` swallow. Errors propagate via set -e.
if ! sysbench --db-driver=mysql \
    --mysql-host=127.0.0.1 --mysql-port="${SERVER_PORT}" \
    --mysql-user=root --mysql-db="${TEST_DB}" \
    --table-size=100 --tables=1 \
    --threads=4 --time=10 \
    oltp_read_only run > "${SYSBENCH_OUT}" 2>&1; then
    assert_fail "sysbench oltp_read_only exit non-zero"
    if [ -f "${SYSBENCH_OUT}" ]; then
        cat "${SYSBENCH_OUT}" >> "${EVIDENCE_FILE}" 2>/dev/null
    fi
    exit 1
else
    if [ -f "${SYSBENCH_OUT}" ]; then
        tail -5 "${SYSBENCH_OUT}" >> "${EVIDENCE_FILE}" 2>/dev/null
    fi
fi
echo ""

echo "[3/4] Point queries (k=42) — 20 iterations"
HIT=0
for _ in $(seq 1 20); do
    RESULT=$(mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -N -B \
        -e "SELECT COUNT(*) FROM sbtest1 WHERE k=42;" | tr -d ' \r')
    if [ "${RESULT}" -ge 1 ]; then
        HIT=$((HIT + 1))
    fi
done
if [ "${HIT}" = "20" ]; then
    assert_pass "20/20 point queries hit (k=42)"
else
    assert_fail "only ${HIT}/20 point queries hit"
    exit 1
fi
echo ""

echo "[4/4] Range scan (k BETWEEN 10 AND 20)"
N=$(mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -N -B \
    -e "SELECT COUNT(*) FROM sbtest1 WHERE k BETWEEN 10 AND 20;" | tr -d ' \r')
if [ "${N}" -ge 1 ] && [ "${N}" -le 100 ]; then
    assert_pass "range scan returned ${N} rows (within [1, 100])"
else
    assert_fail "range scan returned ${N} rows, expected [1, 100]"
    exit 1
fi
echo ""

{
    echo ""
    echo "Result: ${PASS} pass, ${FAIL} fail"
    echo "Finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >> "${EVIDENCE_FILE}"

echo ""
echo "=== E2E sysbench_wired: ${PASS} pass, ${FAIL} fail ==="
[ "${FAIL}" -eq 0 ] && exit 0 || exit 1
