#!/usr/bin/env bash
# V312-57 smoke week01/02: single-path DB + SELECT 1 (--cmd)
# Asserts: sqlrustgo sqlite <db> --cmd "SELECT 1;" exits 0 with "1" in stdout.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/select.db"

OUT=$("$BIN" sqlite --cmd "SELECT 1;" "$DB" 2>&1)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then
    echo "FAIL: exit=$EXIT, got: $OUT"
    exit 1
fi
if ! echo "$OUT" | grep -q '^1$'; then
    echo "FAIL: expected '1' as standalone value, got: $OUT"
    exit 1
fi

echo "PASS: smoke week01/02_select_one"
exit 0