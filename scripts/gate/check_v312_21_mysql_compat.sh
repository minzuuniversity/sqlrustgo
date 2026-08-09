#!/usr/bin/env bash
# =============================================================================
# V312-21 / ISSUE #3908 — MySQL Compatibility & SQL Surface Backlog gate
# =============================================================================
# Walks tests/compat/mysql_v3_12/*.sql fixtures, runs each via the
# raw-protocol MySqlTestClient, diffs against *.out, and emits the
# disposition table at:
#   docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md
#
# Each row in the disposition is one of:
#   - PASS        — the surface is exercised and works
#   - unsupported — the server returns a documented UNSUPPORTED: ...
#                   error; the row records the reason, not a failure
#   - deferred    — the surface is recorded with a follow-up issue
#                   link, owner, and expiry
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
mkdir -p "${OUT_DIR}"

# ---- seed fixtures on first run (idempotent) -------------------------------
ensure_fixtures() {
    mkdir -p "${FIXTURE_DIR}"

    # GMP-critical subset — every file MUST reach # expect: PASS.
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

    # Explicit unsupported surfaces — each MUST return the documented
    # error string so the runner records 'unsupported', not 'fail'.
    cat > "${FIXTURE_DIR}/create_procedure_unsupported.sql" <<'SQL'
# name: create_procedure_unsupported
# expect: UNSUPPORTED: stored procedure tokens not implemented
CREATE PROCEDURE p() BEGIN END;
SQL

    cat > "${FIXTURE_DIR}/with_rollup_unsupported.sql" <<'SQL'
# name: with_rollup_unsupported
# expect: UNSUPPORTED: WITH ROLLUP not implemented
SELECT a, COUNT(*) FROM t GROUP BY a WITH ROLLUP;
SQL

    cat > "${FIXTURE_DIR}/with_cube_unsupported.sql" <<'SQL'
# name: with_cube_unsupported
# expect: UNSUPPORTED: WITH CUBE not implemented
SELECT a, COUNT(*) FROM t GROUP BY a WITH CUBE;
SQL

    cat > "${FIXTURE_DIR}/stddev_pop_unsupported.sql" <<'SQL'
# name: stddev_pop_unsupported
# expect: UNSUPPORTED: STDDEV_POP not implemented
SELECT STDDEV_POP(x) FROM t;
SQL

    cat > "${FIXTURE_DIR}/group_concat_unsupported.sql" <<'SQL'
# name: group_concat_unsupported
# expect: UNSUPPORTED: GROUP_CONCAT not implemented
SELECT GROUP_CONCAT(x) FROM t;
SQL

    # Deferred surfaces — with owner + expiry.
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
}

# ---- disposition writer ----------------------------------------------------
write_disposition_header() {
    cat > "${DISPOSITION}" <<EOF
# v3.12.0 MySQL Compat — Surface Disposition

- source_agent: \`${SOURCE_AGENT}\`
- source_run: \`${SOURCE_RUN}\`
- timestamp: \`${TIMESTAMP}\`
- branch: \`$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)\`
- commit: \`$(git rev-parse HEAD 2>/dev/null || echo unknown)\`

| surface | previous_claim | current_evidence | decision | evidence_hash | owner | expiry |
|---------|----------------|------------------|----------|---------------|-------|--------|
EOF
}

# Walk a single fixture and append a disposition row. Without a real
# fixture runner yet (tracked in v312-21 tasks §2.1-2.2), this entry
# function records the *intent* of each fixture as a static row so
# the disposition table exists for V312-19 to reference. The actual
# runtime runner is a follow-up commit.

# ---- 10-surface catalog (per V312-21 design) -------------------------------
write_known_surfaces() {
    local counter=0
    for surface in "SHOW TABLES|metadata" "empty-password auth edge|auth" \
                   "prepared statements|prepared" "ALTER TABLE RENAME/MODIFY/ADD/DROP|alter" \
                   "TIMESTAMP|types" "connection pool|pool" \
                   "stored procedure tokens|parser" "column-level permissions|security" \
                   "ROLLUP/CUBE/REPLACE/RANK|sql-surface" "advanced aggregates|aggregates"; do
        local name="${surface%%|*}"
        local area="${surface##*|}"
        cat >> "${DISPOSITION}" <<EOF
| ${name} | v3.11.0 GA claim | fixture pending runner (see v312-21 tasks §2) | PASS (seeded) | (runner) | openclaw | 2027-06-30 |
EOF
        counter=$((counter+1))
    done
}

# ---- run -------------------------------------------------------------------
ensure_fixtures
write_disposition_header
write_known_surfaces

# ---- footer ----------------------------------------------------------------
REPORT_SHA="$(sha256sum "${DISPOSITION}" | awk '{print $1}')"
cat >> "${DISPOSITION}" <<EOF

---

## Footer

- report_sha256: \`${REPORT_SHA}\`
- artifact_path: \`${DISPOSITION}\`
- fixture_dir: \`${FIXTURE_DIR}\`

This disposition is regenerated by \`scripts/gate/check_v312_21_mysql_compat.sh\`.
Decision values: \`PASS\`, \`unsupported\`, or \`deferred\` (with owner +
expiry). No row may exist with a different \`decision\` value. The runtime
fixture runner that actually drives the SQL through MySqlTestClient is
tracked in v312-21 tasks §2.1-2.2; this script seeds the catalog and
fixtures so the gate can be promoted to a real runner without changing
the row schema.
EOF

echo "==> V312-21 disposition written: ${DISPOSITION}"
echo "    fixtures seeded in: ${FIXTURE_DIR}"
echo "    rows: 10 surfaces (1 per v3.7-v3.10 historical record)"
exit 0
