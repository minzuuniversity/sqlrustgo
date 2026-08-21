#!/usr/bin/env bash
# V312-57 smoke week06/05: 3-way JOIN + ORDER BY vs SQLite oracle
# Asserts: chained INNER JOINs (3 tables) produce same row multiset as
# SQLite. Catches regressions where join associativity or ON-clause
# scoping breaks beyond the second table.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="$HERE/../lib/sqlite_oracle.sh"
# shellcheck disable=SC1090
. "$LIB"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/mj.db"
SQL="$TMPDIR/script.sql"

cat >"$SQL" <<'EOF'
CREATE TABLE a (id INTEGER, x TEXT);
INSERT INTO a VALUES (1, 'A1');
INSERT INTO a VALUES (2, 'A2');
CREATE TABLE b (id INTEGER, a_id INTEGER, y TEXT);
INSERT INTO b VALUES (10, 1, 'B1');
INSERT INTO b VALUES (11, 2, 'B2');
CREATE TABLE c (id INTEGER, b_id INTEGER, z TEXT);
INSERT INTO c VALUES (100, 10, 'C1');
INSERT INTO c VALUES (101, 11, 'C2');
SELECT a.x, b.y, c.z FROM a JOIN b ON a.id = b.a_id JOIN c ON b.id = c.b_id ORDER BY a.x, b.y, c.z;
EOF

if ! oracle_run "$SQL" "$DB"; then
    echo "FAIL: oracle run reported failure"
    exit 1
fi

echo "PASS: smoke week06/05_multi_join"
exit 0
