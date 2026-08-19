#!/usr/bin/env bash
set -uo pipefail
BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/schema.db"
"$BIN" sqlite --batch "$DB" <<'EOF' > /dev/null 2>&1
CREATE TABLE users (id INTEGER, name TEXT);
EOF
# .schema via REPL (dot-command)
printf '.schema users\n.quit\n' | "$BIN" sqlite "$DB" 2>&1
