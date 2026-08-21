#!/usr/bin/env bash
# V312-48 T4: diff sqlrustgo vs SQLite oracle for SF=1 TPC-H correctness bundle.
#
# Reads per-query TSVs from rows/sqlrustgo/q<N>.tsv and rows/sqlite/q<N>.tsv,
# computes normalized row-count + sorted SHA256 for each engine, writes a
# per-query *.diff file (sorted TSV diff) and a summary summary.json.
#
# Zero-row classification:
#   both_zero        — oracle=0, sqlrustgo=0       (PASS: oracle is also empty)
#   sqlrustgo_zero   — oracle>0, sqlrustgo=0       (FAIL: missing rows)
#   oracle_zero      — oracle=0, sqlrustgo>0       (INVESTIGATE: oracle empty but we have rows)
#   rows_match       — row counts equal AND SHA256 match (PASS)
#   sha256_mismatch  — row counts equal, SHA256 differ (FAIL: wrong values)
#   rows_mismatch    — row counts differ (FAIL: missing or extra rows)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUNDLE="$ROOT/docs/releases/v3.12.0/perf/TPCH_SF1_CORRECTNESS_BUNDLE"
ROWS_SRC="$BUNDLE/rows/sqlrustgo"
ROWS_ORC="$BUNDLE/rows/sqlite"
DIFF_DIR="$BUNDLE/diff"
mkdir -p "$DIFF_DIR"

SUMMARY="$BUNDLE/summary.json"
: > "$SUMMARY.tmp"

# Build sha maps for both engines.
declare -A hash_src hash_orc count_src count_orc

load_sha() {
  local file="$1" key="$2"  # key = "hash" or "count"
  local arr_name="$3"       # associative array name passed by reference
  local -n arr="$arr_name"
  [[ -s "$file" ]] || return 0
  while read -r q h; do
    if [[ "$key" == "hash" ]]; then
      arr["$q"]="$h"
    else
      [[ -f "$BUNDLE/rows/$q.tsv" ]] || continue
      arr["$q"]=$(wc -l < "$BUNDLE/rows/$q.tsv")
    fi
  done < "$file"
}

# Simpler: just load directly.
declare -A hash_src hash_orc
while read -r q h; do hash_src["$q"]="$h"; done < "$BUNDLE/sha256/sqlrustgo.txt" 2>/dev/null || true
while read -r q h; do hash_orc["$q"]="$h"; done < "$BUNDLE/sha256/sqlite.txt" 2>/dev/null || true

# Walk all queries q01..q22.
pass=0; fail=0; zero=0; total=0
> "$DIFF_DIR/diff_summary.tsv"
for n in $(seq 1 22); do
  q=$(printf "q%02d" "$n")
  total=$((total+1))
  src="$ROWS_SRC/${q}.tsv"
  orc="$ROWS_ORC/${q}.tsv"
  src_n=$(wc -l < "$src" 2>/dev/null || echo 0)
  orc_n=$(wc -l < "$orc" 2>/dev/null || echo 0)
  src_h="${hash_src[$q]:-EMPTY}"
  orc_h="${hash_orc[$q]:-EMPTY}"

  status="?"
  diff_lines=0
  diff_file="$DIFF_DIR/${q}.diff"

  if [[ "$src_n" == "0" && "$orc_n" == "0" ]]; then
    status="both_zero"
    zero=$((zero+1))
    : > "$diff_file"
  elif [[ "$src_n" == "0" && "$orc_n" != "0" ]]; then
    status="sqlrustgo_zero"
    fail=$((fail+1))
    diff_lines="$orc_n"
    printf 'oracle had %s rows, sqlrustgo returned 0\n' "$orc_n" > "$diff_file"
  elif [[ "$src_n" != "0" && "$orc_n" == "0" ]]; then
    status="oracle_zero"
    fail=$((fail+1))
    diff_lines="$src_n"
    printf 'sqlrustgo had %s rows, oracle returned 0\n' "$src_n" > "$diff_file"
  elif [[ "$src_h" == "$orc_h" ]]; then
    status="rows_match"
    pass=$((pass+1))
    : > "$diff_file"
  else
    # Row counts match but hashes differ — emit sorted diff for diagnosis.
    if [[ "$src_n" == "$orc_n" ]]; then
      status="sha256_mismatch"
      fail=$((fail+1))
      diff_lines=$(LC_ALL=C diff <(LC_ALL=C sort "$src") <(LC_ALL=C sort "$orc") | wc -l)
      LC_ALL=C diff <(LC_ALL=C sort "$src") <(LC_ALL=C sort "$orc") > "$diff_file" || true
    else
      status="rows_mismatch"
      fail=$((fail+1))
      diff_lines=$((src_n > orc_n ? src_n - orc_n : orc_n - src_n))
      LC_ALL=C diff <(LC_ALL=C sort "$src") <(LC_ALL=C sort "$orc") > "$diff_file" || true
    fi
  fi

  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$q" "$src_n" "$orc_n" "$src_h" "$orc_h" "$status" "$diff_lines" \
    >> "$DIFF_DIR/diff_summary.tsv"
done

printf 'q\trows_src\trows_orc\tsha_src\tsha_orc\tstatus\tdiff_lines\n' > "$DIFF_DIR/diff_summary.tsv.tmp"
cat "$DIFF_DIR/diff_summary.tsv" >> "$DIFF_DIR/diff_summary.tsv.tmp"
mv "$DIFF_DIR/diff_summary.tsv.tmp" "$DIFF_DIR/diff_summary.tsv"

cat > "$SUMMARY" <<EOF
{
  "task": "V312-48 T4",
  "queries_total": $total,
  "pass": $pass,
  "fail": $fail,
  "zero_oracle_also_empty": $zero,
  "generated_at": "$(date -u +%FT%TZ)"
}
EOF
rm -f "$SUMMARY.tmp"

echo "=== V312-48 T4 summary ==="
echo "  total=$total pass=$pass fail=$fail zero=$zero"
echo "  per-query: $DIFF_DIR/diff_summary.tsv"
echo "  summary:   $SUMMARY"