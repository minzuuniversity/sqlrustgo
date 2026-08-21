#!/usr/bin/env bash
# V312-57 smoke week04/02: 错误码稳定 + Error: 前缀
# Asserts: runtime error (column not found) → exit 1 + stable prefix + no panic.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/err.db"

# Setup
"$BIN" sqlite --batch "$DB" <<'EOF' >/dev/null 2>&1
CREATE TABLE t (x INTEGER);
EOF

# Runtime error: column not found
ERR=$("$BIN" sqlite --cmd "SELECT nonexistent_col FROM t;" "$DB" 2>&1 1>/dev/null)
EXIT=$?

if [ "$EXIT" -ne 1 ]; then
    echo "FAIL: expected exit=1, got exit=$EXIT"
    exit 1
fi
if ! echo "$ERR" | grep -qE 'Error:|sqlrustgo:error:'; then
    echo "FAIL: expected stable 'Error:' or 'sqlrustgo:error:' prefix, got: $ERR"
    exit 1
fi
if echo "$ERR" | grep -qi 'panic'; then
    echo "FAIL: panic detected in stderr: $ERR"
    exit 1
fi

echo "PASS: smoke week04/02_stable_error_codes (exit=1, stable prefix)"
exit 0