#!/usr/bin/env bash
# V312-57 smoke week03/02: 跨进程持久化
# Process 1 CREATE+INSERT, Process 2 SELECT (fresh pid) → 'Alice' visible.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/persist.db"

# Process 1: write
"$BIN" sqlite --batch "$DB" <<'EOF' >/dev/null 2>&1
CREATE TABLE users (id INTEGER, name TEXT);
INSERT INTO users VALUES (1, 'Alice');
EOF
P1=$?
if [ "$P1" -ne 0 ]; then echo "FAIL: p1 exit=$P1"; exit 1; fi

# Process 2: read (fresh pid)
OUT=$("$BIN" sqlite --cmd "SELECT name FROM users WHERE id = 1;" "$DB" 2>&1)
P2=$?
if [ "$P2" -ne 0 ]; then echo "FAIL: p2 exit=$P2, stderr=$OUT"; exit 1; fi
if ! echo "$OUT" | grep -q 'Alice'; then
    echo "FAIL: expected 'Alice' in cross-process read, got: $OUT"
    exit 1
fi

echo "PASS: smoke week03/02_cross_process_persistence"
exit 0