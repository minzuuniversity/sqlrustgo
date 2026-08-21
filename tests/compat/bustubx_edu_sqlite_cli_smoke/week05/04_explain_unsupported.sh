#!/usr/bin/env bash
# V312-57 smoke week05/04: EXPLAIN 对不支持语句的稳定错误
# Asserts: `EXPLAIN INSERT INTO ...` exit 1 + stderr contains Error:
# prefix + no panic. Confirms EXPLAIN-via-parser only accepts SELECT /
# ANALYZE / TABLE, and that other DML/DDL produce a stable error prefix
# rather than crashing or panicking.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/explain.db"

# Capture stdout + stderr separately.
"$BIN" sqlite --batch "$DB" <<'EOF' >"$TMPDIR/stdout" 2>"$TMPDIR/stderr"
CREATE TABLE t (x INTEGER);
EXPLAIN INSERT INTO t VALUES (1);
EOF
EXIT=$?

if [ "$EXIT" -ne 1 ]; then
    echo "FAIL: EXPLAIN INSERT exit=$EXIT (expected 1), stdout=$(cat "$TMPDIR/stdout"), stderr=$(cat "$TMPDIR/stderr")"
    exit 1
fi

# Combined stream: at least one of stdout/stderr must contain a stable
# Error prefix.
COMBINED="$(cat "$TMPDIR/stdout" "$TMPDIR/stderr")"
if ! echo "$COMBINED" | grep -q -F -- 'Error:'; then
    echo "FAIL: EXPLAIN INSERT missing 'Error:' prefix, got: $COMBINED"
    exit 1
fi
if echo "$COMBINED" | grep -q 'panic'; then
    echo "FAIL: EXPLAIN INSERT panicked, got: $COMBINED"
    exit 1
fi

echo "PASS: smoke week05/04_explain_unsupported"
exit 0
