#!/usr/bin/env bash
# prepare_sf01_data.sh — Generate SF=0.1 fixture from existing SF=1 data
# Usage: bash scripts/soak/prepare_sf01_data.sh [--output-dir data/tpch-sf01]
#
# Reads SF=1 .tbl files and samples the first N rows to produce SF=0.1 fixtures.
# SF=0.1 row counts (approximate 10% of SF=1):
#   lineitem: 600,000 rows
#   orders:   150,000 rows
#   customer:  15,000 rows
#   part:     20,000 rows
#   partsupp: 80,000 rows
#   supplier:  1,000 rows
#   nation:        25 rows (all)
#   region:         5 rows (all)

set -euo pipefail

OUTPUT_DIR="${1:-data/tpch-sf01}"
SF1_DIR="data/tpch-sf01"

# Row counts for SF=0.1 (10% of SF=1)
declare -A ROW_COUNTS=(
    [lineitem]=600000
    [orders]=150000
    [customer]=15000
    [part]=20000
    [partsupp]=80000
    [supplier]=1000
    [nation]=25
    [region]=5
)

# Full tables (no sampling)
declare -A FULL_TABLES=([nation]=1 [region]=1)

echo "=== TPC-H SF=0.1 Fixture Generator ==="
echo "Output directory: $OUTPUT_DIR"
echo "SF=1 source: $SF1_DIR"
echo ""

# Verify SF=1 data exists
if [[ ! -d "$SF1_DIR" ]]; then
    echo "ERROR: SF=1 data not found at $SF1_DIR"
    exit 1
fi

for tbl in lineitem orders customer part partsupp supplier nation region; do
    src="${SF1_DIR}/${tbl}_clean.tbl"
    dst="${OUTPUT_DIR}/${tbl}_sf01_tbl"
    required_rows="${ROW_COUNTS[$tbl]:-0}"

    if [[ ! -f "$src" ]]; then
        echo "WARNING: $src not found, skipping $tbl"
        continue
    fi

    if [[ -f "$dst" ]]; then
        actual_rows=$(grep -c '' "$dst" 2>/dev/null || echo "0")
        expected_rows=$(( ${required_rows} ))
        if [[ "$actual_rows" -ge "$expected_rows" ]]; then
            echo "  [SKIP] $tbl: already exists ($actual_rows rows >= $expected_rows)"
            continue
        else
            echo "  [REGen] $tbl: exists but has fewer rows ($actual_rows < $expected_rows), regenerating"
            rm -f "$dst"
        fi
    fi

    echo "  [Gen] $tbl: sampling first $required_rows rows from SF=1 ($src)"

    # Use awk for reliable line counting and extraction
    # head -n is too slow for million-line files; awk is faster
    awk "NR >= 1 && NR <= $required_rows" "$src" > "$dst"

    actual=$(grep -c '' "$dst" 2>/dev/null || echo "0")
    echo "         → $dst ($actual rows)"
done

echo ""
echo "=== SF=0.1 Fixture Complete ==="
echo "Files:"
for tbl in lineitem orders customer part partsupp supplier nation region; do
    f="${OUTPUT_DIR}/${tbl}_sf01_tbl"
    if [[ -f "$f" ]]; then
        rows=$(grep -c '' "$f" 2>/dev/null || echo "0")
        size=$(du -sh "$f" 2>/dev/null | cut -f1 || echo "?")
        echo "  $tbl: $rows rows, $size"
    fi
done
