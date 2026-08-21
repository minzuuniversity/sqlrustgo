#!/usr/bin/env bash
# V312-57 smoke week01/03: stdin 批处理
# Asserts: cat script.sql | sqlrustgo sqlite --batch <db> exits 0 with 'Hello' in stdout.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/stdin.db"
SCRIPT="$TMPDIR/script.sql"

cat > "$SCRIPT" <<'EOF'
CREATE TABLE greetings (msg TEXT);
INSERT INTO greetings VALUES ('Hello, world');
SELECT msg FROM greetings;
EOF

OUT=$("$BIN" sqlite --batch "$DB" < "$SCRIPT" 2>&1)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then
    echo "FAIL: exit=$EXIT, got: $OUT"
    exit 1
fi
if ! echo "$OUT" | grep -q 'Hello'; then
    echo "FAIL: expected 'Hello' in output, got: $OUT"
    exit 1
fi

echo "PASS: smoke week01/03_stdin_script"
exit 0