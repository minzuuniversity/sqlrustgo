#!/bin/bash
set -e
cd /home/openclaw/sqlrustgo-sf1-baseline

LOG_DIR=/tmp/tpch-sf1-results
mkdir -p "$LOG_DIR"

for Q in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22; do
  echo "=== Q${Q} ==="
  LOG="$LOG_DIR/q${Q}.log"
  if SQLRUSTGO_WATCHDOG_BYTES=4294967296 TPCH_ONLY_Q=$Q \
       timeout 300 cargo test --release --test tpch_sf1_22_vs_3engines_test \
         "tpch_sf1_22_in_process_regression" -- \
         --include-ignored --nocapture --test-threads=1 \
       > "$LOG" 2>&1; then
    grep -E "Q 2[0-9]:|rows in" "$LOG" | head -2
  else
    echo "Q${Q} FAILED or timed out"
    tail -3 "$LOG"
  fi
  echo "---"
done
