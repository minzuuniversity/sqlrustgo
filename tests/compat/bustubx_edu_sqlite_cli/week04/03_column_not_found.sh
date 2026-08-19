#!/usr/bin/env bash
set -uo pipefail
BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/col.db"
"$BIN" sqlite --batch "$DB" <<'EOF' > /dev/null 2>&1
CREATE TABLE t(a INTEGER);
EOF
# Run SELECT and capture both stdout and stderr — RuntimeError is expected.
"$BIN" sqlite --batch "$DB" <<'EOF' 2>&1
SELECT nonexistent_column FROM t;
EOF
# Exit 0 (we don't fail on the runtime error — we just want to see the
# error message in the golden file).
exit 0
