#!/usr/bin/env bash
# V312-57 smoke week05/02: EXPLAIN SELECT ... WHERE 输出 Filter 节点
# Asserts: WHERE clause is reflected in the plan (Filter node appears).
# This catches the regression class "WHERE is parsed but dropped before
# planning" — common in early-stage executors.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/explain.db"

OUT=$("$BIN" sqlite --batch "$DB" <<'EOF'
CREATE TABLE t (x INTEGER, y TEXT);
INSERT INTO t VALUES (1, 'a');
INSERT INTO t VALUES (2, 'b');
EXPLAIN SELECT * FROM t WHERE x > 1;
EOF
)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then
    echo "FAIL: EXPLAIN SELECT WHERE exit=$EXIT, got: $OUT"
    exit 1
fi

# Either "Filter" (substring) or "x" in the plan (predicate reference)
if ! echo "$OUT" | grep -q -F -- 'Filter'; then
    echo "FAIL: EXPLAIN output missing 'Filter' node for WHERE clause, got: $OUT"
    exit 1
fi

echo "PASS: smoke week05/02_explain_select_filter"
exit 0
