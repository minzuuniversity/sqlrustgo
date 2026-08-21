#!/usr/bin/env bash
# V312-57 smoke week02/02: --mode csv 输出
# Asserts: CSV header 'id,name' + Alice value present.
# NOTE: develop's `--headers` is `Option<bool>` (clap ValueEnum), so it only
# accepts `--headers true` or `--headers false`. We use explicit column names
# because develop's CSV formatter does not expand `SELECT *` to column names
# (it would emit a literal `*` header).

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/csv.db"

OUT=$("$BIN" sqlite --batch --mode csv --headers true "$DB" <<'EOF'
CREATE TABLE u (id INTEGER, name TEXT);
INSERT INTO u VALUES (1, 'Alice');
SELECT id, name FROM u;
EOF
)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then
    echo "FAIL: exit=$EXIT, got: $OUT"
    exit 1
fi
if ! echo "$OUT" | grep -q 'id,name'; then
    echo "FAIL: expected 'id,name' CSV header, got: $OUT"
    exit 1
fi
if ! echo "$OUT" | grep -q 'Alice'; then
    echo "FAIL: expected 'Alice' in CSV row, got: $OUT"
    exit 1
fi

echo "PASS: smoke week02/02_csv_output"
exit 0