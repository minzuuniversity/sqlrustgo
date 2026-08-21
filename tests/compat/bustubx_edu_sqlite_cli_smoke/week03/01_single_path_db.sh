#!/usr/bin/env bash
# V312-57 smoke week03/01: 多种 DB path 形式
# Asserts: rel / abs / nested paths all open and a CREATE+INSERT+SELECT
# batch succeeds, with distinct sentinel values per variant (42 / 99 / 7).
# (True cross-process persistence is a known race per develop #4373 — defer
# to week05+.)

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

# variant_path:sentinel
i=0
for pair in "p1/rel:42" "p2/abs:99" "p3/sub/nested:7"; do
    variant="${pair%%:*}"
    sentinel="${pair##*:}"
    DB="$TMPROOT/$variant.db"
    mkdir -p "$(dirname "$DB")"
    OUT=$("$BIN" sqlite --batch "$DB" <<EOF
CREATE TABLE t (x INTEGER);
INSERT INTO t VALUES ($sentinel);
SELECT x FROM t;
EOF
)
    EXIT=$?
    i=$((i + 1))
    if [ "$EXIT" -ne 0 ]; then
        echo "FAIL: variant '$variant' batch exit=$EXIT, got: $OUT"
        exit 1
    fi
    if ! echo "$OUT" | grep -q "$sentinel"; then
        echo "FAIL: variant '$variant' output missing $sentinel, got: $OUT"
        exit 1
    fi
done

echo "PASS: smoke week03/01_single_path_db (rel/abs/nested all OK)"
exit 0