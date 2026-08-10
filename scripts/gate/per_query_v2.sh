#!/bin/bash
# Per-query SF=1.0 isolation test with 4GB watchdog
set -e

# cd to repo root (detect from script location)
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
LOG_DIR="/tmp/tpch-per-query-v2"
mkdir -p "$LOG_DIR"

# 4GB watchdog
LIMIT_GB=4
INTERVAL=2
GRACE=30

for Q in $(seq 1 22); do
  echo "=== Q${Q} ==="
  LOG="$LOG_DIR/q${Q}.log"
  bash scripts/gate/watchdog_memory.sh \
    --limit-gb $LIMIT_GB --interval $INTERVAL --grace $GRACE -- \
    env SQLRUSTGO_WATCHDOG_BYTES=$((LIMIT_GB * 1024 * 1024 * 1024)) \
        TPCH_ONLY_Q=$Q \
    cargo test --release --test tpch_sf1_22_vs_3engines_test \
      "tpch_sf1_22_in_process_regression\[Q${Q}\]" -- \
      --include-ignored --nocapture --test-threads=1 \
    2>&1 | tee "$LOG" | grep -E '^\[watchdog\]|rows in|rows\]|^   test |ok$|^FAILED'
  echo "---"
done
