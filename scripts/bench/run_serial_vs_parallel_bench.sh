#!/bin/bash
# run_serial_vs_parallel_bench.sh — Issue #3792
# Usage: ./run_serial_vs_parallel_bench.sh [sf] [degrees] [runs]
#
# Examples:
#   ./run_serial_vs_parallel_bench.sh          # SF=0.1, degrees=1,4, runs=3
#   ./run_serial_vs_parallel_bench.sh 1 1,4,8 3 # Full SF, all degrees, 3 runs

set -e

SF="${1:-0.1}"
DEGREES="${2:-1,4}"
RUNS="${3:-3}"
OUTPUT_DIR="${OUTPUT_DIR:-/tmp/svp_reports}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== Serial vs Parallel Benchmark ==="
echo "SF=$SF, Degrees=$DEGREES, Runs=$RUNS"
echo "Output: $OUTPUT_DIR"
echo ""

mkdir -p "$OUTPUT_DIR"

# Quick smoke test (SF=0.1, degrees 1,4)
cargo run --example serial_vs_parallel_bench \
    -- \
    --sf 0.1 \
    --degrees "1,4" \
    --runs 1 \
    --quick \
    --output-dir "$OUTPUT_DIR" \
    2>&1 | tee "$OUTPUT_DIR/quick_sf01.log"

echo ""
echo "=== Quick smoke test complete ==="

# Full SF=0.1 comparison
if [[ "$SF" != "0.1" ]] || [[ "$DEGREES" != "1,4" ]]; then
    echo ""
    echo "=== Running full SF=$SF benchmark ==="
    cargo run --example serial_vs_parallel_bench \
        -- \
        --sf "$SF" \
        --degrees "$DEGREES" \
        --runs "$RUNS" \
        --output-dir "$OUTPUT_DIR" \
        2>&1 | tee "$OUTPUT_DIR/full_sf${SF}.log"
fi

# Print summary
echo ""
echo "=== Benchmark complete ==="
echo "Reports in: $OUTPUT_DIR"
ls -lh "$OUTPUT_DIR"/*.json "$OUTPUT_DIR"/*.md 2>/dev/null || true
