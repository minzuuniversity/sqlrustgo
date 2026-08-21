#!/usr/bin/env bash
# V312-57 smoke week06/01: INNER JOIN 2 tables vs SQLite oracle
# Asserts: row multiset of `INNER JOIN` matches SQLite 3.51+ output. This
# is the foundational JOIN smoke — catches the regression class "JOIN
# silently degenerates to cartesian product" or "ON clause ignored".

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="$HERE/../lib/sqlite_oracle.sh"
# shellcheck disable=SC1090
. "$LIB"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/join.db"
SQL="$TMPDIR/script.sql"

cat >"$SQL" <<'EOF'
CREATE TABLE users (id INTEGER, name TEXT);
INSERT INTO users VALUES (1, 'Alice');
INSERT INTO users VALUES (2, 'Bob');
CREATE TABLE orders (id INTEGER, user_id INTEGER, amount INTEGER);
INSERT INTO orders VALUES (100, 1, 50);
INSERT INTO orders VALUES (101, 1, 75);
INSERT INTO orders VALUES (102, 2, 30);
SELECT u.id, u.name, o.amount FROM users u INNER JOIN orders o ON u.id = o.user_id;
EOF

if ! oracle_run "$SQL" "$DB"; then
    echo "FAIL: oracle run reported failure"
    exit 1
fi

echo "PASS: smoke week06/01_join_inner"
exit 0
