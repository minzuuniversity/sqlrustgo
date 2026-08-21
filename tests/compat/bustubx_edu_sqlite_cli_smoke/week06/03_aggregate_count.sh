#!/usr/bin/env bash
# V312-57 smoke week06/03: COUNT(*) WHERE vs SQLite oracle
# Asserts: scalar COUNT(*) with WHERE predicate matches SQLite. Catches
# regressions where COUNT ignores WHERE (counts entire table) or returns
# NULL/0 instead of the actual count.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="$HERE/../lib/sqlite_oracle.sh"
# shellcheck disable=SC1090
. "$LIB"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/cnt.db"
SQL="$TMPDIR/script.sql"

cat >"$SQL" <<'EOF'
CREATE TABLE t (id INTEGER, cat TEXT);
INSERT INTO t VALUES (1, 'A');
INSERT INTO t VALUES (2, 'A');
INSERT INTO t VALUES (3, 'B');
INSERT INTO t VALUES (4, 'A');
INSERT INTO t VALUES (5, 'B');
SELECT COUNT(*) FROM t WHERE cat = 'A';
EOF

if ! oracle_run "$SQL" "$DB"; then
    echo "FAIL: oracle run reported failure"
    exit 1
fi

echo "PASS: smoke week06/03_aggregate_count"
exit 0
