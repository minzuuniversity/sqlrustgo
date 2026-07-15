#!/usr/bin/env bash
# RC E2E scenario: sysbench_wired
# Run a sysbench OLTP-read-only workload against sqlrustgo's MySQL wire
# protocol to validate end-to-end throughput.
#
# If `sysbench` is not installed (CI runners often lack it), fall back to a
# simple multi-statement OLTP-read loop using the `mysql` client, which
# exercises the same code path (wire protocol + executor + storage).
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_sysbench_$$"
PASS=0
FAIL=0

cleanup() {
    mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root \
        -e "DROP DATABASE IF EXISTS $TEST_DB;" 2>/dev/null || true
}
trap cleanup EXIT

assert_pass() { PASS=$((PASS+1)); echo "  ✅ $1"; }
assert_fail() { FAIL=$((FAIL+1)); echo "  ❌ $1"; }

echo "=== E2E sysbench_wired ==="
echo "Server: 127.0.0.1:$SERVER_PORT"
echo "Test DB: $TEST_DB"
echo ""

echo "[1/4] Prepare: create sbtest1 with 100 rows"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE $TEST_DB;" 2>/dev/null
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" -e "
CREATE TABLE sbtest1 (
    id INT PRIMARY KEY,
    k INT NOT NULL,
    c VARCHAR(64) NOT NULL,
    pad VARCHAR(64) NOT NULL
);
" 2>/dev/null
for i in $(seq 1 100); do
    mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" \
        -e "INSERT INTO sbtest1 VALUES ($i, $((RANDOM % 1000)), 'payload-$i', 'pad-$i');" 2>/dev/null
done
N=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" -N -B \
    -e "SELECT COUNT(*) FROM sbtest1;" 2>/dev/null | tr -d ' \r')
if [ "$N" = "100" ]; then
    assert_pass "sbtest1 prepared with 100 rows"
else
    assert_fail "sbtest1 has $N rows, expected 100"
    exit 1
fi
echo ""

if command -v sysbench >/dev/null 2>&1; then
    echo "[2/4] Run sysbench oltp_read_only (10s, 4 threads)"
    sysbench --db-driver=mysql \
        --mysql-host=127.0.0.1 --mysql-port="$SERVER_PORT" \
        --mysql-user=root --mysql-db="$TEST_DB" \
        --table-size=100 --tables=1 \
        --threads=4 --time=10 \
        oltp_read_only run > /tmp/sysbench_$$.out 2>&1 || true
    if grep -q "transactions:" /tmp/sysbench_$$.out 2>/dev/null; then
        TPS=$(grep "transactions:" /tmp/sysbench_$$.out | awk '{print $3}' | tr -d '()' || echo "0")
        assert_pass "sysbench oltp_read_only completed (≈$ TPS TPS)"
    else
        assert_fail "sysbench output did not contain transactions (see /tmp/sysbench_$$.out)"
    fi
    rm -f /tmp/sysbench_$$.out
else
    echo "[2/4] sysbench not installed — falling back to 200-statement OLTP read loop"
    OK=0
    for i in $(seq 1 200); do
        K=$((RANDOM % 100 + 1))
        RESULT=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" -N -B \
            -e "SELECT c, pad FROM sbtest1 WHERE k=$K LIMIT 1;" 2>/dev/null | tr -d ' \r')
        if [ -n "$RESULT" ]; then
            OK=$((OK + 1))
        fi
    done
    if [ "$OK" -ge 180 ]; then
        assert_pass "OLTP read fallback: $OK/200 successful (>=180)"
    else
        assert_fail "OLTP read fallback: only $OK/200 succeeded"
    fi
fi
echo ""

echo "[3/4] Point queries (k=42) — 20 iterations"
HIT=0
for _ in $(seq 1 20); do
    R=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" -N -B \
        -e "SELECT id, c FROM sbtest1 WHERE k=42;" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$R" -ge 1 ]; then HIT=$((HIT+1)); fi
done
if [ "$HIT" = "20" ]; then
    assert_pass "Point query k=42 hit 20/20"
else
    assert_fail "Point query k=42 hit $HIT/20"
fi
echo ""

echo "[4/4] Range scan (k BETWEEN 10 AND 20)"
N=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root "$TEST_DB" -N -B \
    -e "SELECT COUNT(*) FROM sbtest1 WHERE k BETWEEN 10 AND 20;" 2>/dev/null | tr -d ' \r')
if [ "$N" -ge 1 ] && [ "$N" -le 100 ]; then
    assert_pass "Range scan returned $N rows (within expected bounds)"
else
    assert_fail "Range scan returned $N rows, out of bounds"
fi
echo ""

echo "=== E2E sysbench_wired: $PASS pass, $FAIL fail ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
