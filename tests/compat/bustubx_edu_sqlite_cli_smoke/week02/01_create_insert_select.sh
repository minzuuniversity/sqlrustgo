#!/usr/bin/env bash
# V312-57 smoke week02/01: CREATE TABLE + INSERT + SELECT
# Asserts: --batch stdin script succeeds and 'Alice' appears.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/crud.db"

OUT=$("$BIN" sqlite --batch "$DB" <<'EOF'
CREATE TABLE users (id INTEGER, name TEXT);
INSERT INTO users VALUES (1, 'Alice');
INSERT INTO users VALUES (2, 'Bob');
SELECT name FROM users ORDER BY id;
EOF
)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then
    echo "FAIL: exit=$EXIT, got: $OUT"
    exit 1
fi
if ! echo "$OUT" | grep -q 'Alice'; then
    echo "FAIL: expected 'Alice' in output, got: $OUT"
    exit 1
fi
if ! echo "$OUT" | grep -q 'Bob'; then
    echo "FAIL: expected 'Bob' in output, got: $OUT"
    exit 1
fi

echo "PASS: smoke week02/01_create_insert_select"
exit 0