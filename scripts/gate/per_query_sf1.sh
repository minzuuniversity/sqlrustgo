#!/bin/bash
# Per-query SF=1.0 isolation test — runs each query in its own cargo test invocation
set -e

# watchdog_memory.sh skipped (WD variable not defined)
TPCH_SF1_DIR=/tmp/tpch-sf1
TPCH_BINT_DIR=/tmp/tpch-sf1-bin
TPCH_SF1_SQLRUSTGO_DATA_DIR=/tmp/tpch-sf1
LIMIT_GB=4
INTERVAL=2
GRACE=30

OUT_DIR="${OUT_DIR:-/tmp/tpch-per-query}"
mkdir -p "$OUT_DIR"

total_mem_kb=$(grep MemTotal /proc/meminfo | awk '{print $2}')
total_mem_gb=$(echo "scale=1; $total_mem_kb / 1024 / 1024" | bc)
echo "System total memory: ${total_mem_gb} GB"
echo "Per-query limit: ${LIMIT_GB} GB"
echo "Output dir: $OUT_DIR"
echo ""

for Q in $(seq 1 22); do
  echo "=== Q${Q} ==="
  LOG="$OUT_DIR/q${Q}.log"
  # Run with test filter so ONLY this query executes
  bash scripts/gate/watchdog_memory.sh \
    --limit-gb $LIMIT_GB --interval $INTERVAL --grace $GRACE -- \
    cargo test --release --test tpch_sf1_22_vs_3engines_test \
    "tpch_sf1_22_in_process_regression\[Q${Q}\]" -- \
    --include-ignored --nocapture --test-threads=1 \
    2>&1 | tee "$LOG" | grep -E '^\[watchdog\]|rows in|rows\]|^   test |ok$|^FAILED'
  echo "---"
done
