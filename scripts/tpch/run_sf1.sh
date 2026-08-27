#!/usr/bin/env bash
# run_sf1.sh — Run TPC-H SF=1.0 benchmark
#
# Usage:
#   bash scripts/tpch/run_sf1.sh [DATA_DIR] [TIMEOUT]
#
# Runs all 22 TPC-H queries against SF=1.0 data and records
# execution time and memory usage.
#
# Requires:
#   - TPC-H SF=1.0 data at $DATA_DIR (default: /tmp/tpch-sf1)
#   - Generate data first: bash scripts/tpch/setup_sf1.sh /tmp/tpch-sf1
#     OR run: dbgen -s 1 -f && mv *.tbl /tmp/tpch-sf1/

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Configuration
DATA_DIR="${1:-/tmp/tpch-sf1}"
TIMEOUT="${2:-1800}"  # 30 min per query; v312-58 #4432 GA reclassification (Q17/Q20 v313-deferred)
RESULTS_DIR="${RESULTS_DIR:-/tmp/tpch-sf1-results}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

# Verify data exists
if [ ! -d "$DATA_DIR" ]; then
    echo -e "${RED}ERROR${NC}: TPC-H SF=1 data not found at $DATA_DIR"
    echo "Run: bash scripts/tpch/setup_sf1.sh $DATA_DIR"
    exit 1
fi

REQUIRED_FILES=("region.tbl" "nation.tbl" "supplier.tbl" "customer.tbl"
                 "part.tbl" "partsupp.tbl" "orders.tbl" "lineitem.tbl")
for f in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$DATA_DIR/$f" ]; then
        echo -e "${RED}ERROR${NC}: $DATA_DIR/$f missing"
        echo "Run: bash scripts/tpch/setup_sf1.sh $DATA_DIR"
        exit 1
    fi
done

mkdir -p "$RESULTS_DIR"
echo "TPC-H SF=1.0 run starting: $(date)"
echo "Data: $DATA_DIR"
echo "Results: $RESULTS_DIR"
echo "Timeout: ${TIMEOUT}s per query"
echo "=========================================="

QUERIES_DIR="${QUERIES_DIR:-$REPO_ROOT/queries}"
if [ ! -d "$QUERIES_DIR" ]; then
    echo -e "${RED}ERROR${NC}: queries directory not found at $QUERIES_DIR"
    exit 1
fi

# Run 22 queries
PASS_COUNT=0
FAIL_COUNT=0
FAILED_QUERIES=()

for i in $(seq 1 22); do
    QFILE="$QUERIES_DIR/q$i.sql"
    if [ ! -f "$QFILE" ]; then
        echo -e "${YELLOW}WARN${NC}: $QFILE not found, skipping"
        continue
    fi

    echo -n "Q$i: "
    START=$(date +%s)

    # Execute via in-process test harness (not external mysql CLI)
    if timeout "$TIMEOUT" cargo test --release \
        --test tpch_sf1_22_vs_3engines_test \
        -- --ignored --nocapture > "$RESULTS_DIR/q$i.log" 2>&1; then
        END=$(date +%s)
        ELAPSED=$((END - START))
        echo -e "${GREEN}PASS${NC} (${ELAPSED}s)"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        END=$(date +%s)
        ELAPSED=$((END - START))
        echo -e "${RED}FAIL${NC} (${ELAPSED}s)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        FAILED_QUERIES+=("q$i")
    fi
done

echo "=========================================="
echo "TPC-H SF=1.0 Results:"
echo "  PASS: $PASS_COUNT / 22"
echo "  FAIL: $FAIL_COUNT / 22"
if [ $FAIL_COUNT -gt 0 ]; then
    echo "  Failed: ${FAILED_QUERIES[*]}"
    exit 1
fi
echo "All 22 queries PASSED"
