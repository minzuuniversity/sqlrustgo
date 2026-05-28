#!/usr/bin/env bash
# Benchmark Suite for SQLRustGo v3.6.0
# Architecture Convergence Release
#
# Usage: ./run_bench.sh [output.json]
#
# Outputs benchmark results in JSON format for CI regression detection.

set -e

OUTPUT="${1:-benchmark.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== SQLRustGo v3.6.0 Benchmark Suite ==="
echo "Output: $OUTPUT"
echo ""

# Ensure we're in the right directory
cd "$ROOT_DIR"

# Check if we have benches
if [ ! -d "benches" ]; then
    echo "Error: benches/ directory not found"
    exit 1
fi

# Run cargo bench with JSON output
# Note: We use criterion's JSON output format
echo "Running benchmarks..."
echo ""

# Create output with metadata
cat > "$OUTPUT" << 'EOF'
{
  "version": "3.6.0",
  "timestamp": "EOF

echo -n $(date -u +%Y-%m-%dT%H:%M:%SZ) >> "$OUTPUT"

cat >> "$OUTPUT" << 'EOF',
  "commit": "EOF

echo -n $(git rev-parse HEAD 2>/dev/null || echo "unknown") >> "$OUTPUT"

cat >> "$OUTPUT" << 'EOF',
  "benchmarks": [
EOF

# Run individual benchmarks and capture results
# We run a subset of critical benchmarks

BENCH_NAMES=(
    "bench_scan::seq_scan_1000"
    "bench_scan::range_scan_1000"
    "bench_aggregate::sum_i32_10000"
    "bench_aggregate::avg_f64_10000"
    "bench_insert::insert_100"
)

FIRST=true
for bench in "${BENCH_NAMES[@]}"; do
    echo "  Running: $bench"

    # Run the specific bench and capture time
    result=$(cargo bench -- "$bench" --noplot 2>/dev/null | grep -E "time:|Mean" | head -3 || echo "")

    if [ -n "$result" ]; then
        # Extract mean time in nanoseconds
        time_ns=$(echo "$result" | grep "Mean" | awk '{print $2}' | tr -d 'ns' || echo "0")

        if [ "$FIRST" = true ]; then
            FIRST=false
        else
            echo "," >> "$OUTPUT"
        fi

        cat >> "$OUTPUT" << EOF
    {
      "name": "$bench",
      "unit": "ns",
      "mean": $time_ns
    }
EOF
    fi
done

cat >> "$OUTPUT" << 'EOF'

  ]
}
EOF

echo ""
echo "=== Benchmark Complete ==="
echo "Results written to: $OUTPUT"
echo ""

# Print summary
if command -v python3 &> /dev/null; then
    python3 -c "
import json
with open('$OUTPUT') as f:
    data = json.load(f)
print('Benchmarks run:', len(data['benchmarks']))
for b in data['benchmarks']:
    print(f'  {b[\"name\"]}: {b[\"mean\"]} ns')
" 2>/dev/null || true
fi
