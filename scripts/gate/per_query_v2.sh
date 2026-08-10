#!/bin/bash
# Per-query TPC-H isolation test with 4GB watchdog.
# Supports --sf N arg (per corpus_manifest.yaml tpch_sf10 entry). When --sf is
# not 1, the script emits a clear `deferred` status (no SF=10 test exists yet)
# with explicit reason in the log, so the corpus runner can record the
# honest result instead of silently running SF=1 tests under a SF=10 label.
set -e

# Parse args
SF=1
while [[ $# -gt 0 ]]; do
    case "$1" in
        --sf) SF="$2"; shift 2 ;;
        --sf=*) SF="${1#*=}"; shift ;;
        *) echo "Unknown arg: $1" >&2; exit 2 ;;
    esac
done

# cd to repo root (detect from script location)
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
LOG_DIR="/tmp/tpch-per-query-v2"
mkdir -p "$LOG_DIR"

# 4GB watchdog
LIMIT_GB=4
INTERVAL=2
GRACE=30

if [ "$SF" != "1" ]; then
    # V312-46: only SF=1 test exists. For SF>1 (e.g. --sf 10), emit explicit
    # deferred marker so corpus runner records honest reason.
    echo "deferred: per_query_v2.sh --sf $SF not supported (only SF=1 test exists at tests/integration/tpch_sf1_22_vs_3engines_test)"
    echo "  Follow-up: add tests/integration/tpch_sf10_22_vs_3engines_test (per V312-19 R2.7 follow-up)"
    exit 0
fi

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
