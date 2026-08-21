#!/usr/bin/env bash
# V312-57 smoke week03/03: .tables meta-command (REPL)
# Asserts: CREATE TABLE → .tables output contains table name.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/tables.db"

# Setup tables in batch mode
"$BIN" sqlite --batch "$DB" <<'EOF' >/dev/null 2>&1
CREATE TABLE users (id INTEGER);
CREATE TABLE orders (id INTEGER);
EOF

# Query .tables via REPL
OUT=$(printf '.tables\n.quit\n' | "$BIN" sqlite "$DB" 2>&1)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then echo "FAIL: exit=$EXIT, got: $OUT"; exit 1; fi
if ! echo "$OUT" | grep -q 'users'; then
    echo "FAIL: expected 'users' in .tables output, got: $OUT"
    exit 1
fi
if ! echo "$OUT" | grep -q 'orders'; then
    echo "FAIL: expected 'orders' in .tables output, got: $OUT"
    exit 1
fi

echo "PASS: smoke week03/03_tables_meta"
exit 0