#!/bin/bash
# Setup TPC-H SF=0.1 data for in-process testing (Sprint 5 / Issue #3302 follow-up).
#
# The .tbl files are gitignored (8MB).  This script copies them from the
# canonical TPC-H data location (where the perf team generated them) into
# tests/data/tpch-sf01/.  The in-process test in
# tests/tpch_sf01_inprocess_test.rs SKIPs itself if the data is missing.
#
# Source: /System/Volumes/Data/private/tmp/tpch_sf01/  (8 tables, ~9MB total)
#   region.tbl   5 rows       nation.tbl    25 rows
#   supplier.tbl 100 rows     customer.tbl  1500 rows
#   part.tbl     2000 rows    partsupp.tbl  8000 rows
#   orders.tbl   15000 rows   lineitem.tbl  60000 rows
#
# If dbgen is available locally, regenerate instead of copying:
#   dbgen -s 0.1 -f
#   mv *.tbl tests/data/tpch-sf01/

set -euo pipefail

SRC="${TPCH_SF01_SRC:-/System/Volumes/Data/private/tmp/tpch_sf01}"
DST="$(cd "$(dirname "$0")/.." && pwd)/tests/data/tpch-sf01"

if [[ ! -d "$SRC" ]]; then
    echo "[ERROR] Source not found: $SRC" >&2
    echo "        Set TPCH_SF01_SRC to the directory containing 8 .tbl files" >&2
    exit 1
fi

mkdir -p "$DST"

EXPECTED_TABLES=(region nation supplier customer part partsupp orders lineitem)
copied=0
for t in "${EXPECTED_TABLES[@]}"; do
    if [[ -f "$SRC/$t.tbl" ]]; then
        cp -f "$SRC/$t.tbl" "$DST/$t.tbl"
        rows=$(wc -l < "$DST/$t.tbl" | tr -d ' ')
        echo "  $t.tbl: $rows rows"
        copied=$((copied + 1))
    else
        echo "[WARN] missing $t.tbl in $SRC" >&2
    fi
done

# Clean up runtime artifacts that some previous run might have left
rm -f "$DST"/*.json "$DST"/*.wal 2>/dev/null || true

echo
echo "[OK] $copied/8 tables staged in $DST"
echo "     Run:  cargo test --test tpch_sf01_inprocess_test --all-features -- --nocapture"
echo "     All:  TPCH_SF01_ALL=1 cargo test --test tpch_sf01_inprocess_test --all-features -- --nocapture"
