#!/usr/bin/env bash
# V312-57 smoke week04/01: .schema meta-command (REPL)
# Asserts: .schema <table> shows table name.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/schema.db"

# Setup
"$BIN" sqlite --batch "$DB" <<'EOF' >/dev/null 2>&1
CREATE TABLE users (id INTEGER, name TEXT);
EOF

# Query .schema users
OUT=$(printf '.schema users\n.quit\n' | "$BIN" sqlite "$DB" 2>&1)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then echo "FAIL: exit=$EXIT, got: $OUT"; exit 1; fi
if ! echo "$OUT" | grep -qi 'users'; then
    echo "FAIL: expected 'users' in .schema output, got: $OUT"
    exit 1
fi

echo "PASS: smoke week04/01_schema_meta"
exit 0