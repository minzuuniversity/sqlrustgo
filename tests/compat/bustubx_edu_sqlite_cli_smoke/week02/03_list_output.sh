#!/usr/bin/env bash
# V312-57 smoke week02/03: --mode list 输出 (pipe-separated)
# Asserts: row format '2|Bob' present.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/list.db"

OUT=$("$BIN" sqlite --batch --mode list "$DB" <<'EOF'
CREATE TABLE v (id INTEGER, name TEXT);
INSERT INTO v VALUES (2, 'Bob');
SELECT * FROM v;
EOF
)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then
    echo "FAIL: exit=$EXIT, got: $OUT"
    exit 1
fi
if ! echo "$OUT" | grep -q '2|Bob'; then
    echo "FAIL: expected '2|Bob' in list mode, got: $OUT"
    exit 1
fi

echo "PASS: smoke week02/03_list_output"
exit 0