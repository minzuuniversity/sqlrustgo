#!/usr/bin/env bash
# V312-48 T3: SQLite oracle for SF=1 TPC-H correctness bundle.
#
# Loads /tmp/tpch-sf1/*.tbl into a fresh SQLite DB at
# docs/releases/v3.12.0/perf/TPCH_SF1_CORRECTNESS_BUNDLE/oracle.sqlite,
# runs queries/q{1..22}.sql, dumps TSV + SHA256 into the bundle.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SF1="${TPCH_SF1_DIR:-/tmp/tpch-sf1}"
BUNDLE="$ROOT/docs/releases/v3.12.0/perf/TPCH_SF1_CORRECTNESS_BUNDLE"
DB="$BUNDLE/oracle.sqlite"
rm -rf "$DB" "$BUNDLE/rows/sqlite" "$BUNDLE/sha256"
mkdir -p "$BUNDLE/rows/sqlite" "$BUNDLE/sha256"

sqlite3 "$DB" < "$ROOT/scripts/sqlite_tpch_setup.sql"

for tbl in region nation supplier customer part partsupp orders lineitem; do
  echo "Loading $tbl ..."
  sqlite3 "$DB" <<SQL
.mode list
.separator "|"
.import $SF1/$tbl.tbl $tbl
SQL
  count=$(sqlite3 "$DB" "SELECT count(*) FROM $tbl;")
  echo "  $tbl=$count"
done

# Speed up the JOIN-heavy queries (q17 was timing out without idx_lineitem_partkey).
sqlite3 "$DB" <<'SQL'
CREATE INDEX IF NOT EXISTS idx_lineitem_partkey     ON lineitem(l_partkey);
CREATE INDEX IF NOT EXISTS idx_lineitem_quantity     ON lineitem(l_quantity);
CREATE INDEX IF NOT EXISTS idx_lineitem_discount     ON lineitem(l_discount);
CREATE INDEX IF NOT EXISTS idx_lineitem_extendedprice ON lineitem(l_extendedprice);
CREATE INDEX IF NOT EXISTS idx_part_brand           ON part(p_brand);
CREATE INDEX IF NOT EXISTS idx_part_container       ON part(p_container);
SQL

> "$BUNDLE/sha256/sqlite.txt"
for n in $(seq 1 22); do
  q=$(printf "q%02d" "$n")
  out="$BUNDLE/rows/sqlite/${q}.tsv"
  # SQLite needs strftime, not PostgreSQL EXTRACT.
  sql=$(sed -e "s/EXTRACT(YEAR FROM \\([^)]*\\))/CAST(strftime('%Y', \\1) AS INTEGER)/g" \
        "$ROOT/queries/q${n}.sql")
  printf "%s\n" "$sql" | sqlite3 -separator $'\t' -header "$DB" \
    | tail -n +2 > "$out" || true
  if [[ ! -s "$out" ]]; then
    : > "$out"
  fi
  h=$(LC_ALL=C sort "$out" | sha256sum | awk '{print $1}')
  echo "$q $h" >> "$BUNDLE/sha256/sqlite.txt"
done
sort -o "$BUNDLE/sha256/sqlite.txt" "$BUNDLE/sha256/sqlite.txt"

echo "=== SQLite oracle SHA256 ==="
cat "$BUNDLE/sha256/sqlite.txt"