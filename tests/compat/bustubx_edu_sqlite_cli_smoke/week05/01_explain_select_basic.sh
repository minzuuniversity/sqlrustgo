#!/usr/bin/env bash
# V312-57 smoke week05/01: EXPLAIN SELECT 输出查询计划
# Asserts: `EXPLAIN SELECT * FROM t` exit 0 + stdout contains plan keywords
# (SeqScan / Projection / Filter) — proves EXPLAIN syntax is plumbed through
# to the planner, not silently swallowed.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/explain.db"

OUT=$("$BIN" sqlite --batch "$DB" <<'EOF'
CREATE TABLE t (x INTEGER, y TEXT);
INSERT INTO t VALUES (1, 'a');
INSERT INTO t VALUES (2, 'b');
EXPLAIN SELECT * FROM t;
EOF
)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then
    echo "FAIL: EXPLAIN SELECT exit=$EXIT, got: $OUT"
    exit 1
fi

# The planner emits at minimum a scan node + projection for a SELECT *.
# Accept any of the three plan keywords — different versions name them
# slightly differently.
HITS=0
for kw in SeqScan Projection Filter; do
    if echo "$OUT" | grep -q -F -- "$kw"; then
        HITS=$((HITS + 1))
    fi
done
if [ "$HITS" -lt 1 ]; then
    echo "FAIL: EXPLAIN output missing plan keywords (SeqScan/Projection/Filter), got: $OUT"
    exit 1
fi

echo "PASS: smoke week05/01_explain_select_basic (hits=$HITS)"
exit 0
