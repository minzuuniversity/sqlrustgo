#!/usr/bin/env bash
# V312-57 smoke week06/02: SUM(...) GROUP BY vs SQLite oracle
# Asserts: aggregate SUM over GROUP BY produces the same row multiset as
# SQLite. Catches regressions where aggregate is parsed but executed as
# identity (returns first row), or where GROUP BY is dropped (single
# summed row).

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="$HERE/../lib/sqlite_oracle.sh"
# shellcheck disable=SC1090
. "$LIB"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/agg.db"
SQL="$TMPDIR/script.sql"

cat >"$SQL" <<'EOF'
CREATE TABLE orders (user_id INTEGER, amount INTEGER);
INSERT INTO orders VALUES (1, 50);
INSERT INTO orders VALUES (1, 75);
INSERT INTO orders VALUES (2, 30);
INSERT INTO orders VALUES (2, 20);
INSERT INTO orders VALUES (3, 100);
SELECT user_id, SUM(amount) FROM orders GROUP BY user_id;
EOF

if ! oracle_run "$SQL" "$DB"; then
    echo "FAIL: oracle run reported failure"
    exit 1
fi

echo "PASS: smoke week06/02_aggregate_sum"
exit 0
