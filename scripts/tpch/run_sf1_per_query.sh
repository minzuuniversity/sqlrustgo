#!/usr/bin/env bash
# Run TPC-H SF=1 queries one at a time with per-query timeout.
# Usage: bash scripts/tpch/run_sf1_per_query.sh [START_Q] [END_Q]
set -uo pipefail

START_Q="${1:-1}"
END_Q="${2:-22}"
PER_Q_TIMEOUT="${TPCH_PER_Q_TIMEOUT:-1800}"  # v312-58 #4432 GA reclassification (Q17/Q20 v313-deferred)

echo "=== TPC-H SF=1 per-query runner (Q${START_Q}..Q${END_Q}, timeout ${PER_Q_TIMEOUT}s each) ==="

for q in $(seq "${START_Q}" "${END_Q}"); do
    echo ""
    echo "--- Q${q} ---"
    if timeout "${PER_Q_TIMEOUT}" env TPCH_ONLY_Q="${q}" \
        cargo test --release --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture 2>&1 \
        | grep -E "Q [0-9]+:|query complete|skipped"; then
        echo "Q${q}: completed (or skipped)"
    else
        echo "Q${q}: FAILED or timed out"
    fi
done