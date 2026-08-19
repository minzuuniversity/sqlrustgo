#!/usr/bin/env bash
set -uo pipefail
BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/err.db"

# Set up a table for runtime errors
"$BIN" sqlite --batch "$DB" <<'EOF' > /dev/null 2>&1
CREATE TABLE x(a INTEGER);
EOF

# ParseError
printf 'SELEC 1\n.quit\n' | "$BIN" sqlite "$DB" 2>&1

# IoError (.read missing file)
printf '.read /nonexistent.sql\n.quit\n' | "$BIN" sqlite "$DB" 2>&1

# DotCmdError (unknown dot-command)
printf '.foo\n.quit\n' | "$BIN" sqlite "$DB" 2>&1
