#!/usr/bin/env bash
# V312-57 smoke week05/03: EXPLAIN SELECT ... JOIN 输出连接节点
# Asserts: an EXPLAIN over a 2-table JOIN produces a join node in the plan
# (NestedLoopJoin / HashJoin / MergeJoin). This exercises more of the
# planner than a single-table EXPLAIN — catches regressions where join
# reordering or join type selection silently drops the join node.
#
# NOTE: develop does NOT support EXPLAIN TABLE or EXPLAIN ANALYZE — those
# are reserved for the RC upgrade track. We test EXPLAIN SELECT with
# various plan-node kinds instead.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/explain.db"

OUT=$("$BIN" sqlite --batch "$DB" <<'EOF'
CREATE TABLE a (id INTEGER, x TEXT);
CREATE TABLE b (id INTEGER, a_id INTEGER, y TEXT);
INSERT INTO a VALUES (1, 'A1');
INSERT INTO b VALUES (10, 1, 'B1');
EXPLAIN SELECT a.x, b.y FROM a JOIN b ON a.id = b.a_id;
EOF
)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then
    echo "FAIL: EXPLAIN SELECT JOIN exit=$EXIT, got: $OUT"
    exit 1
fi

# The plan must contain at least one join-node keyword.
HITS=0
for kw in NestedLoopJoin HashJoin MergeJoin Join; do
    if echo "$OUT" | grep -q -F -- "$kw"; then
        HITS=$((HITS + 1))
    fi
done
if [ "$HITS" -lt 1 ]; then
    echo "FAIL: EXPLAIN JOIN output missing join-node keyword (NestedLoopJoin/HashJoin/MergeJoin), got: $OUT"
    exit 1
fi

echo "PASS: smoke week05/03_explain_select_join (hits=$HITS)"
exit 0
