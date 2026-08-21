#!/usr/bin/env bash
# V312-57 smoke week01/05: parse 错误退出码 = 1 + Error: 前缀 + 无 panic
# Asserts: 'SELEC 1;' (typo) → exit 1, stderr 'Error: ...', no panic.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/parse.db"

ERR=$("$BIN" sqlite --cmd "SELEC 1;" "$DB" 2>&1 1>/dev/null)
EXIT=$?

if [ "$EXIT" -ne 1 ]; then
    echo "FAIL: expected exit=1, got exit=$EXIT, stderr: $ERR"
    exit 1
fi
if ! echo "$ERR" | grep -qE 'Error:|sqlrustgo:error:'; then
    echo "FAIL: expected 'Error:' or 'sqlrustgo:error:' in stderr, got: $ERR"
    exit 1
fi
if echo "$ERR" | grep -qi 'panic'; then
    echo "FAIL: panic detected in stderr: $ERR"
    exit 1
fi

echo "PASS: smoke week01/05_exit_code_parse_error (exit=1, stable Error prefix)"
exit 0