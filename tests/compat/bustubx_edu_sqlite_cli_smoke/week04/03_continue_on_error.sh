#!/usr/bin/env bash
# V312-57 smoke week04/03: --continue-on-error 不中断后续语句
# Asserts: 3rd statement (after parse error) actually executes + persists.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/cont.db"

# 3 statements: OK / parse_err / OK
"$BIN" sqlite --continue-on-error --batch "$DB" <<'EOF' >/dev/null 2>&1
CREATE TABLE t (x INTEGER);
INSERT INTO t VALUES (1);
SELEC 2;
INSERT INTO t VALUES (3);
EOF

# Verify 3rd statement ran by reading from a fresh query
OUT=$("$BIN" sqlite --cmd "SELECT x FROM t ORDER BY x;" "$DB" 2>&1)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then echo "FAIL: verify exit=$EXIT, got: $OUT"; exit 1; fi
if ! echo "$OUT" | grep -q '1'; then
    echo "FAIL: value 1 missing (1st INSERT not persisted): $OUT"
    exit 1
fi
if ! echo "$OUT" | grep -q '3'; then
    echo "FAIL: value 3 missing (--continue-on-error did NOT run 3rd stmt): $OUT"
    exit 1
fi

echo "PASS: smoke week04/03_continue_on_error"
exit 0