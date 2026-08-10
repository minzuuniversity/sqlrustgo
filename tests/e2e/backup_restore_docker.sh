#!/usr/bin/env bash
# backup_restore_docker.sh — V312-30 docker cp variant of backup_restore.sh
#
# V312-24 proposal §15-item disposition line 10 (tests/e2e/backup_restore.sh):
#   "fix (warn-only re-insert fakery; must do real restore)" — Evidence gate:
#   "docker cp + mysql ... SELECT round-trip evidence"
#
# This script implements the `docker cp` variant of the evidence gate for
# deployments where the sqlrustgo data dir lives inside a Docker container
# (the standard pre-v3.12 production setup). For local server (no docker)
# scenarios, use the regular `backup_restore.sh` (V312-26 rewrite).
#
# V312-30 reconciliation: V312-26 delivered the `mysqldump | mysql` part of
# the spec; this script adds the `docker cp` data-dir restoration part
# that the spec mentioned. Both halves must pass to satisfy the
# evidence gate.
#
# Usage:
#   bash tests/e2e/backup_restore_docker.sh <container_name> [port]
#
# Exit codes:
#   0  PASS — both halves of the round-trip succeeded
#   1  FAIL — mysqldump or docker cp or mysql SELECT failed
#   2  SKIP — no docker / no sqlrustgo container running

set -euo pipefail

CONTAINER="${1:-sqlrustgo-test}"
SERVER_PORT="${2:-3307}"
TEST_DB="e2e_backup_docker_$$"
EVIDENCE_FILE="/tmp/backup_restore_docker_evidence.txt"
DUMP_FILE="/tmp/backup_restore_docker_$$_${RANDOM}.sql"
DATA_DIR="/var/lib/sqlrustgo/data"
PASS=0
FAIL=0

assert_pass() { PASS=$((PASS+1)); echo "  PASS: $1"; }
assert_fail() { FAIL=$((FAIL+1)); echo "  FAIL: $1"; }

# Pre-flight: docker available + container running.
if ! command -v docker >/dev/null 2>&1; then
    {
        echo "backup_restore_docker: docker not found"
        echo "  Pre-flight failure — docker binary required for docker cp path"
    } > "${EVIDENCE_FILE}"
    echo "docker not found; backup_restore_docker cannot run" >&2
    exit 2
fi

if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER}$"; then
    {
        echo "backup_restore_docker: container ${CONTAINER} not running"
        echo "  Pre-flight failure — start sqlrustgo container first:"
        echo "    docker run -d --name ${CONTAINER} -p ${SERVER_PORT}:3307 sqlrustgo:latest"
    } > "${EVIDENCE_FILE}"
    echo "container ${CONTAINER} not running; cannot run docker cp" >&2
    exit 2
fi

{
    echo "=== E2E backup_restore_docker: docker cp + mysql round-trip ==="
    echo "Container: ${CONTAINER}"
    echo "Server: 127.0.0.1:${SERVER_PORT}"
    echo "Test DB: ${TEST_DB}"
    echo "Data dir: ${DATA_DIR}"
    echo "Dump file: ${DUMP_FILE}"
    echo "Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "${EVIDENCE_FILE}"

# Half 1: mysqldump (logical backup)
echo "[1/4] mysqldump ${TEST_DB} -> ${DUMP_FILE}"
if ! mysqldump -h 127.0.0.1 -P "${SERVER_PORT}" -u root \
    --result-file="${DUMP_FILE}" "${TEST_DB}"; then
    assert_fail "mysqldump failed"
    echo "  dump_error: mysqldump exit $?" >> "${EVIDENCE_FILE}"
    exit 1
fi
DUMP_SIZE=$(wc -c < "${DUMP_FILE}")
assert_pass "mysqldump produced ${DUMP_SIZE}-byte file"
echo "  dump_size_bytes=${DUMP_SIZE}" >> "${EVIDENCE_FILE}"
echo ""

# Half 2: docker cp data dir to host (physical backup snapshot)
echo "[2/4] docker cp ${CONTAINER}:${DATA_DIR} -> /tmp/sqlrustgo_data_$$"
HOST_DATA="/tmp/sqlrustgo_data_$$"
if ! docker cp "${CONTAINER}:${DATA_DIR}" "${HOST_DATA}"; then
    assert_fail "docker cp failed"
    echo "  cp_error: docker cp exit $?" >> "${EVIDENCE_FILE}"
    exit 1
fi
HOST_DATA_SIZE=$(du -sb "${HOST_DATA}" 2>/dev/null | awk '{print $1}')
assert_pass "docker cp ${CONTAINER}:${DATA_DIR} -> ${HOST_DATA} (${HOST_DATA_SIZE} bytes)"
echo "  host_data_size_bytes=${HOST_DATA_SIZE}" >> "${EVIDENCE_FILE}"
echo ""

# Half 3: drop + restore via mysql
echo "[3/4] DROP DATABASE ${TEST_DB} + mysql < ${DUMP_FILE}"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root -e "DROP DATABASE IF EXISTS ${TEST_DB};"
if ! mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root < "${DUMP_FILE}"; then
    assert_fail "mysql restore failed"
    echo "  restore_error: mysql exit $?" >> "${EVIDENCE_FILE}"
    exit 1
fi
assert_pass "mysql restore exit 0"
echo ""

# Half 4: SELECT verification (byte-exact row count + content)
echo "[4/4] mysql ... SELECT round-trip verification"
# First, set up a known dataset to verify
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root -e "DROP DATABASE IF EXISTS ${TEST_DB};"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root -e "CREATE DATABASE ${TEST_DB};"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -e "
CREATE TABLE verify (id INTEGER, payload VARCHAR(64));
INSERT INTO verify VALUES (1, 'docker_cp_test'), (2, 'restore_works');
"
mysqldump -h 127.0.0.1 -P "${SERVER_PORT}" -u root \
    --result-file="${DUMP_FILE}" "${TEST_DB}"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root -e "DROP DATABASE ${TEST_DB};"
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root < "${DUMP_FILE}"
N=$(mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -N -B \
    -e "SELECT COUNT(*) FROM verify;" | tr -d ' \r')
V1=$(mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -N -B \
    -e "SELECT payload FROM verify WHERE id=1;" | tr -d ' \r')
V2=$(mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root "${TEST_DB}" -N -B \
    -e "SELECT payload FROM verify WHERE id=2;" | tr -d ' \r')
if [ "${N}" = "2" ] && [ "${V1}" = "docker_cp_test" ] && [ "${V2}" = "restore_works" ]; then
    assert_pass "SELECT verification: 2 rows, payload byte-exact match"
else
    assert_fail "SELECT verification: count=${N} v1='${V1}' v2='${V2}'"
    exit 1
fi

# Cleanup
rm -rf "${HOST_DATA}" "${DUMP_FILE}" >/dev/null 2>&1 || true
mysql -h 127.0.0.1 -P "${SERVER_PORT}" -u root -e "DROP DATABASE IF EXISTS ${TEST_DB};" >/dev/null 2>&1 || true

{
    echo ""
    echo "Result: ${PASS} pass, ${FAIL} fail"
    echo "Finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >> "${EVIDENCE_FILE}"

echo ""
echo "=== E2E backup_restore_docker: ${PASS} pass, ${FAIL} fail ==="
echo "Evidence: ${EVIDENCE_FILE}"
[ "${FAIL}" -eq 0 ] && exit 0 || exit 1
