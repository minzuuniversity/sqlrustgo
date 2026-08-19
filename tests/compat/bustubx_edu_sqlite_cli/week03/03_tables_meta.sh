#!/usr/bin/env bash
set -uo pipefail
BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/meta.db"
# Set up tables
"$BIN" sqlite --batch "$DB" <<'EOF' > /dev/null 2>&1
CREATE TABLE users (id INTEGER, name TEXT);
CREATE TABLE orders (id INTEGER, total REAL);
INSERT INTO users VALUES (1, 'alice');
EOF
# Use REPL with stdin to test .tables (REPL accepts dot-commands)
printf '.tables\n.quit\n' | "$BIN" sqlite "$DB" 2>&1
