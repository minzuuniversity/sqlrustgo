#!/usr/bin/env bash
# V312-57 smoke week01/04: 正常退出码 = 0
# Asserts: sqlrustgo sqlite --cmd "SELECT 42;" <db> exits 0.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/exit.db"

"$BIN" sqlite --cmd "SELECT 42;" "$DB" >/dev/null 2>&1
EXIT=$?

if [ "$EXIT" -ne 0 ]; then
    echo "FAIL: expected exit=0, got exit=$EXIT"
    exit 1
fi

echo "PASS: smoke week01/04_exit_code_success (exit=0)"
exit 0