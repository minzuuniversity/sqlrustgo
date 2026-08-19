#!/usr/bin/env bash
set -uo pipefail
BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/edu.db"
"$BIN" sqlite --batch "$DB" <<'EOF'
CREATE TABLE only_one(x INTEGER);
INSERT INTO only_one VALUES (42);
EOF
# Verify DB path exists as a directory with the expected table file
# (timestamp-independent — just check file existence, not metadata).
test -d "$DB" && test -f "$DB/only_one.json" && echo "OK: only_one.json exists in DB directory"
