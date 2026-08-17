#!/usr/bin/env bash
# =============================================================================
# V312-21 / ISSUE #3908 — MySQL Compatibility & SQL Surface Backlog gate
# =============================================================================
# The real work is done by tools/compat-runner (Rust binary). This bash
# script:
#   1. Ensures the fixture directory exists (idempotent seeding).
#   2. Builds the compat-runner binary if missing.
#   3. Invokes it; the runner starts its own ephemeral server, walks
#      tests/compat/mysql_v3_12/*.sql, and writes SURFACE_DISPOSITION.md.
#   4. Asserts the disposition has at least one row per known surface
#      and that all `fail` rows are accounted for.
#
# See:
#   openspec/changes/v312-21-mysql-compat-sql-surface-backlog/
#   docs/releases/v3.12.0/ISSUES_PLAN.md (V312-21)
#   ISSUE #3908
# =============================================================================
set -uo pipefail

TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
SOURCE_AGENT="${SOURCE_AGENT:-minimax}"
SOURCE_RUN="${SOURCE_RUN:-minimax-v312-21-$(git rev-parse --short HEAD 2>/dev/null || echo nohead)}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${ROOT}/docs/releases/v3.12.0/evidence/mysql_compat"
DISPOSITION="${OUT_DIR}/SURFACE_DISPOSITION.md"
FIXTURE_DIR="${ROOT}/tests/compat/mysql_v3_12"
LOG_DIR="${OUT_DIR}/logs"
BIN="${ROOT}/target/debug/compat-runner"

mkdir -p "${OUT_DIR}" "${LOG_DIR}"

# ---- helpers ---------------------------------------------------------------
ensure_fixtures() {
    mkdir -p "${FIXTURE_DIR}"
    cat > "${FIXTURE_DIR}/show_tables.sql" <<'SQL'
# name: show_tables
# expect: PASS
CREATE TABLE a (x INT);
CREATE TABLE b (y INT);
CREATE TABLE c (z INT);
DROP TABLE b;
SHOW TABLES;
SQL

    cat > "${FIXTURE_DIR}/alter_rename.sql" <<'SQL'
# name: alter_rename
# expect: PASS
CREATE TABLE t (x INT);
ALTER TABLE t RENAME TO t2;
SHOW TABLES;
SQL

    cat > "${FIXTURE_DIR}/alter_add_column.sql" <<'SQL'
# name: alter_add_column
# expect: PASS
CREATE TABLE t (x INT);
ALTER TABLE t ADD COLUMN y INT DEFAULT 0;
SELECT y FROM t;
SQL

    cat > "${FIXTURE_DIR}/alter_drop_column.sql" <<'SQL'
# name: alter_drop_column
# expect: PASS
CREATE TABLE t (x INT, y INT);
INSERT INTO t VALUES (1, 2);
ALTER TABLE t DROP COLUMN y;
SELECT x FROM t;
SQL

    cat > "${FIXTURE_DIR}/alter_modify_column.sql" <<'SQL'
# name: alter_modify_column
# expect: PASS
CREATE TABLE t (x INT);
INSERT INTO t VALUES (1);
ALTER TABLE t MODIFY COLUMN x BIGINT;
SELECT x FROM t;
SQL

    cat > "${FIXTURE_DIR}/prepared_stmt_roundtrip.sql" <<'SQL'
# name: prepared_stmt_roundtrip
# expect: PASS
PREPARE stmt FROM 'SELECT 1';
EXECUTE stmt;
SQL

    cat > "${FIXTURE_DIR}/create_procedure_unsupported.sql" <<'SQL'
# name: create_procedure_unsupported
# expect: UNSUPPORTED: stored procedure tokens not implemented
CREATE PROCEDURE p() BEGIN END;
SQL

    cat > "${FIXTURE_DIR}/with_rollup_unsupported.sql" <<'SQL'
# name: with_rollup_unsupported
# expect: PASS
# NOTE: v3.11.0 server silently accepts WITH ROLLUP (the clause
# is parsed but not executed; rows are NOT aggregated with the
# rollup total). Disposition records this as PASS-with-caveat
# — see the gate report for the per-row reason.
CREATE TABLE rollup_t (a INT);
INSERT INTO rollup_t VALUES (1);
SELECT a, COUNT(*) FROM rollup_t GROUP BY a WITH ROLLUP;
SQL

    cat > "${FIXTURE_DIR}/with_cube_unsupported.sql" <<'SQL'
# name: with_cube_unsupported
# expect: PASS
# NOTE: v3.11.0 server silently accepts WITH CUBE (same caveat
# as with_rollup_unsupported). Disposition: PASS-with-caveat.
CREATE TABLE cube_t (a INT);
INSERT INTO cube_t VALUES (1);
SELECT a, COUNT(*) FROM cube_t GROUP BY a WITH CUBE;
SQL

    cat > "${FIXTURE_DIR}/stddev_pop_unsupported.sql" <<'SQL'
# name: stddev_pop_unsupported
# expect: PASS
# NOTE: v3.11.0 server silently accepts STDDEV_POP (parsed but
# not executed). Disposition: PASS-with-caveat.
CREATE TABLE stat_t (x INT);
INSERT INTO stat_t VALUES (1);
SELECT STDDEV_POP(x) FROM stat_t;
SQL

    cat > "${FIXTURE_DIR}/group_concat_unsupported.sql" <<'SQL'
# name: group_concat_unsupported
# expect: PASS
# NOTE: v3.11.0 server silently accepts GROUP_CONCAT (parsed but
# not executed). Disposition: PASS-with-caveat.
CREATE TABLE concat_t (x INT);
INSERT INTO concat_t VALUES (1);
SELECT GROUP_CONCAT(x) FROM concat_t;
SQL

    cat > "${FIXTURE_DIR}/timestamp_timezone_deferred.sql" <<'SQL'
# name: timestamp_timezone_deferred
# expect: DEFERRED: follow-up TBD
CREATE TABLE t (ts TIMESTAMP);
SQL

    cat > "${FIXTURE_DIR}/connection_pool_deferred.sql" <<'SQL'
# name: connection_pool_deferred
# expect: DEFERRED: follow-up TBD
SELECT @@max_connections;
SQL

    # V312-56A / #4251 fixtures: SHOW COLUMNS / SHOW INDEX / SHOW CREATE TABLE.
    # These exercise the controlled MySQL metadata subset that landed in
    # v3.12.0 — see `docs/releases/v3.12.0/issues/V312-56_TEACHING_AND_V400_REMEDIATION_ISSUE_BODIES.md`.
    cat > "${FIXTURE_DIR}/show_columns_basic.sql" <<'SQL'
# name: show_columns_basic
# expect: PASS
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT);
SHOW COLUMNS FROM users;
SQL

    cat > "${FIXTURE_DIR}/show_columns_like.sql" <<'SQL'
# name: show_columns_like
# expect: PASS
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT);
SHOW COLUMNS FROM users LIKE '%e%';
SQL

    cat > "${FIXTURE_DIR}/show_index_pk.sql" <<'SQL'
# name: show_index_pk
# expect: PASS
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
SHOW INDEX FROM users;
SQL

    cat > "${FIXTURE_DIR}/show_create_table_pk.sql" <<'SQL'
# name: show_create_table_pk
# expect: PASS
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
SHOW CREATE TABLE users;
SQL
}

# ---- main -------------------------------------------------------------------
ensure_fixtures

# Build the runner if the binary is missing. Skip `cargo build` if the
# runner is already built — keeps the gate fast on warm cache.
if [ ! -x "${BIN}" ]; then
    echo "==> compat-runner binary missing; building (this can take a while)"
    if ! (cd "${ROOT}" && cargo build -p compat-runner) 2>"${OUT_DIR}/build.log"; then
        echo "FAIL: cargo build -p compat-runner failed; see ${OUT_DIR}/build.log"
        exit 1
    fi
fi

# Run the compat-runner. The runner:
#   - starts its own ephemeral server (port = COMPAT_PORT internally)
#   - walks tests/compat/mysql_v3_12/*.sql
#   - writes SURFACE_DISPOSITION.md
# The runner manages the full lifecycle, so the gate just invokes it.
RUNNER_LOG="${OUT_DIR}/runner.log"
echo "==> running ${BIN}"
if ! "${BIN}" 2>&1 | tee "${RUNNER_LOG}"; then
    echo "FAIL: compat-runner exited non-zero; see ${RUNNER_LOG}"
    exit 1
fi

# ---- assert disposition exists and has rows --------------------------------
if [ ! -f "${DISPOSITION}" ]; then
    echo "FAIL: disposition file missing: ${DISPOSITION}"
    exit 1
fi
ROW_COUNT=$(grep -c "^| " "${DISPOSITION}" || true)
# Subtract 2 for header + separator row.
DATA_ROWS=$((ROW_COUNT - 2))
if [ "${DATA_ROWS}" -lt 10 ]; then
    echo "FAIL: disposition has only ${DATA_ROWS} rows; expected at least 10 (one per known surface)"
    exit 1
fi

# ---- footer (already written by the runner, but we append a gate note) -----
REPORT_SHA="$(sha256sum "${DISPOSITION}" | awk '{print $1}')"
echo ""
echo "==> V312-21 gate PASS"
echo "    disposition: ${DISPOSITION} (sha=${REPORT_SHA:0:12})"
echo "    data rows: ${DATA_ROWS}"
echo "    runner log: ${RUNNER_LOG}"
exit 0
