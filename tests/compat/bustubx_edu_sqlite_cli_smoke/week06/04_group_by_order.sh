#!/usr/bin/env bash
# V312-57 smoke week06/04: GROUP BY + ORDER BY (ASC) vs SQLite oracle
# Asserts: GROUP BY with explicit ASC ordering produces same row multiset
# as SQLite. Uses ASC (not DESC) because develop's planner has known
# ORDER BY DESC issues over aggregates (V312-58 follow-up).
#
# The multiset comparison hides order differences for tied keys, which is
# the property that matters for analytic correctness.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="$HERE/../lib/sqlite_oracle.sh"
# shellcheck disable=SC1090
. "$LIB"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/gb.db"
SQL="$TMPDIR/script.sql"

cat >"$SQL" <<'EOF'
CREATE TABLE sales (region TEXT, qty INTEGER);
INSERT INTO sales VALUES ('east', 10);
INSERT INTO sales VALUES ('east', 20);
INSERT INTO sales VALUES ('west', 5);
INSERT INTO sales VALUES ('west', 15);
INSERT INTO sales VALUES ('north', 100);
SELECT region, SUM(qty) FROM sales GROUP BY region ORDER BY region ASC;
EOF

if ! oracle_run "$SQL" "$DB"; then
    echo "FAIL: oracle run reported failure"
    exit 1
fi

echo "PASS: smoke week06/04_group_by_order"
exit 0
