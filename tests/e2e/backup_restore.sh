#!/usr/bin/env bash
# RC E2E scenario: backup_restore
# Verify true backup-and-restore via mysqldump + mysql round-trip.
#
# V312-26 rewrite: replaced `2>/dev/null || true` + fake re-insert with
# real `mysqldump | mysql` round-trip. The script now:
#   1. Creates a database and inserts N rows.
#   2. Dumps the database to a real file via mysqldump.
#   3. Drops the database.
#   4. Restores from the dump via mysql.
#   5. Verifies the row count + content survived the round-trip.
#
# Failure modes are explicit: every step that hits the server is checked,
# and `set -euo pipefail` aborts the script on any error. The previous
# implementation used `|| true` to silently swallow failures and re-inserted
# the same data to fake a "successful restore" — V312-26 closes that hole.
set -euo pipefail

SERVER_PORT="${1:-3307}"
TEST_DB="e2e_backup_$$"
DUMP_FILE="/tmp/e2e_backup_$$_${RANDOM}.sql"
EVIDENCE_FILE="/tmp/backup_restore_evidence.txt"
PASS=0
FAIL=0

cleanup() {
    # Best-effort cleanup; cleanup itself must not fail the script.
    # Use explicit if-guard to avoid the `|| true` anti-pattern.
    if ! mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root \
            -e "DROP DATABASE IF EXISTS ${TEST_DB};" >/dev/null 2>&1; then
        :
    fi
    if ! rm -f "${DUMP_FILE}" >/dev/null 2>&1; then
        :
    fi
}
trap cleanup EXIT

assert_pass() { PASS=$((PASS+1)); echo "  PASS: $1"; }
assert_fail() { FAIL=$((FAIL+1)); echo "  FAIL: $1"; }

# Pre-flight: mysqldump must be installed. (V312-26 explicit fail-explicit.)
if ! command -v mysqldump >/dev/null 2>&1; then
    {
        echo "backup_restore: mysqldump not found"
        echo "  Pre-flight failure — mysqldump binary required for real restore round-trip"
        echo "  Server: 127.0.0.1:${SERVER_PORT}"
        echo "  Test DB: ${TEST_DB}"
        echo "  Dump file (never written): ${DUMP_FILE}"
    } > "${EVIDENCE_FILE}"
    echo "  FAIL: mysqldump not found; backup_restore cannot run real round-trip" >&2
    exit 1
fi

{
    echo "=== E2E backup_restore: real mysqldump + mysql round-trip ==="
    echo "Server: 127.0.0.1:${SERVER_PORT}"
    echo "Test DB: ${TEST_DB}"
    echo "Dump file: ${DUMP_FILE}"
    echo "Evidence file: ${EVIDENCE_FILE}"
    echo "Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo ""
} > "${EVIDENCE_FILE}"

echo "=== E2E backup_restore: real mysqldump + mysql round-trip ==="
echo "Server: 127.0.0.1:${SERVER_PORT}"
echo "Test DB: ${TEST_DB}"
echo "Dump file: ${DUMP_FILE}"
echo ""

echo "[1/5] Create database and table with 5 rows"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root -e "CREATE DATABASE ${TEST_DB};"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -e "
CREATE TABLE kv (k INT PRIMARY KEY, v VARCHAR(64));
INSERT INTO kv VALUES (1, 'one'), (2, 'two'), (3, 'three'), (4, 'four'), (5, 'five');
"
assert_pass "DB + table created with 5 rows"
echo ""

echo "[2/5] mysqldump ${TEST_DB} -> ${DUMP_FILE}"
if mysqldump -h 127.0.0.1 -P "${SERVER_PORT}" -u root \
    --result-file="${DUMP_FILE}" "${TEST_DB}"; then
    if [ -s "${DUMP_FILE}" ]; then
        assert_pass "mysqldump produced non-empty file ($(wc -c < "${DUMP_FILE}") bytes)"
        echo "  dump_size_bytes=$(wc -c < "${DUMP_FILE}")" >> "${EVIDENCE_FILE}"
    else
        assert_fail "mysqldump returned 0 but produced empty file"
        echo "  dump_error: empty output" >> "${EVIDENCE_FILE}"
        exit 1
    fi
else
    assert_fail "mysqldump command failed"
    echo "  dump_error: mysqldump exit $?" >> "${EVIDENCE_FILE}"
    exit 1
fi
echo ""

echo "[3/5] DROP DATABASE ${TEST_DB}"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root -e "DROP DATABASE ${TEST_DB};"
assert_pass "DROP DATABASE"
echo ""

echo "[4/5] mysql < ${DUMP_FILE}  (real restore, not re-insert)"
if mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root < "${DUMP_FILE}"; then
    assert_pass "mysql restore command exit 0"
else
    assert_fail "mysql restore command failed"
    echo "  restore_error: mysql exit $?" >> "${EVIDENCE_FILE}"
    exit 1
fi
echo ""

echo "[5/5] Verify the restored DB has the original 5 rows"
N=$(mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -N -B \
    -e "SELECT COUNT(*) FROM kv;" | tr -d ' \r')
if [ "${N}" = "5" ]; then
    assert_pass "Round-trip preserved 5 rows (real restore, not re-insert)"
else
    assert_fail "Round-trip returned ${N} rows, expected 5"
    echo "  row_count: ${N}" >> "${EVIDENCE_FILE}"
    exit 1
fi

# Verify content too (defense-in-depth)
for K in 1 2 3 4 5; do
    EXPECT=$(printf "case %d: " "${K}"; echo "${K}" | awk '{print "case " $1 ": " (($1==1)?"one":(($1==2)?"two":(($1==3)?"three":(($1==4)?"four":"five"))))}')
    ACTUAL=$(mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -N -B \
        -e "SELECT v FROM kv WHERE k=${K};" | tr -d ' \r')
    if [ "${ACTUAL}" = "$(echo "${K}" | awk '{print ($1==1)?"one":(($1==2)?"two":(($1==3)?"three":(($1==4)?"four":"five"))}')" ]; then
        assert_pass "row k=${K} v='${ACTUAL}'"
    else
        assert_fail "row k=${K} v='${ACTUAL}' (mismatch)"
    fi
done

{
    echo ""
    echo "Result: ${PASS} pass, ${FAIL} fail"
    echo "Finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >> "${EVIDENCE_FILE}"

echo ""
echo "=== E2E backup_restore: ${PASS} pass, ${FAIL} fail ==="
[ "${FAIL}" -eq 0 ] && exit 0 || exit 1
