#!/usr/bin/env bash
# Benchmark Gate CI Script
# Fails if QPS drops > 15% from baseline
#
# Usage: ./check_benchmark_gate.sh [baseline.json] [current.json]
#
# Environment variables:
#   REGRESSION_THRESHOLD - threshold percentage (default: 15)
#   BASELINE_DIR - directory containing baseline results (default: .benchmarks)

set -e

BASELINE="${1:-.benchmarks/baseline.json}"
CURRENT="${2:-benchmark.json}"
THRESHOLD="${REGRESSION_THRESHOLD:-15}"
BASELINE_DIR="${BASELINE_DIR:-.benchmarks}"

echo "=== SQLRustGo v3.6.0 Benchmark Gate ==="
echo "Baseline: $BASELINE"
echo "Current:  $CURRENT"
echo "Threshold: ${THRESHOLD}%"
echo ""

# Create baseline directory if it doesn't exist
mkdir -p "$BASELINE_DIR"

# Check if baseline exists
if [ ! -f "$BASELINE" ]; then
    echo "⚠️  Baseline file not found: $BASELINE"
    echo "    This appears to be the first run. Creating baseline..."
    echo ""
    if [ -f "$CURRENT" ]; then
        cp "$CURRENT" "$BASELINE"
        echo "✅ Baseline created: $BASELINE"
        echo "    This baseline will be used for future comparisons."
        exit 0
    else
        echo "❌ No current benchmark results found either."
        echo "    Run 'cargo bench' first to generate results."
        exit 1
    fi
fi

# Check if current results exist
if [ ! -f "$CURRENT" ]; then
    echo "❌ Current benchmark results not found: $CURRENT"
    echo "    Run 'cargo bench' or ./run_bench.sh first."
    exit 1
fi

# Parse and compare benchmarks using Python
python3 << 'EOF'
import json
import sys
import os

baseline_file = os.environ.get('BASELINE', '.benchmarks/baseline.json')
current_file = os.environ.get('CURRENT', 'benchmark.json')
threshold = float(os.environ.get('THRESHOLD', '15'))

try:
    with open(baseline_file) as f:
        baseline = json.load(f)
    with open(current_file) as f:
        current = json.load(f)
except FileNotFoundError as e:
    print(f"Error: {e}")
    sys.exit(1)
except json.JSONDecodeError as e:
    print(f"Error: Invalid JSON in benchmark file: {e}")
    sys.exit(1)

baseline_map = {b['name']: b for b in baseline.get('benchmarks', [])}
current_map = {b['name']: b for b in current.get('benchmarks', [])}

total_regressions = 0
total_improvements = 0
regression_list = []
improvement_list = []

for name, baseline_bench in baseline_map.items():
    if name not in current_map:
        print(f"⚠️  Benchmark '{name}' not found in current results")
        continue

    current_bench = current_map[name]
    baseline_mean = float(baseline_bench.get('mean', 0))
    current_mean = float(current_bench.get('mean', 0))

    if baseline_mean == 0:
        print(f"⚠️  Baseline mean is 0 for '{name}', skipping")
        continue

    # For time-based benchmarks, an increase is a regression
    # For QPS-based benchmarks, a decrease is a regression
    change_pct = ((current_mean - baseline_mean) / baseline_mean) * 100

    if change_pct > threshold:
        total_regressions += 1
        regression_list.append({
            'name': name,
            'baseline': baseline_mean,
            'current': current_mean,
            'change': change_pct
        })
    elif change_pct < -threshold:
        total_improvements += 1
        improvement_list.append({
            'name': name,
            'baseline': baseline_mean,
            'current': current_mean,
            'change': change_pct
        })

print("=" * 60)
print("BENCHMARK GATE RESULTS")
print("=" * 60)
print()

if regression_list:
    print(f"❌ REGRESSIONS DETECTED: {total_regressions}")
    print()
    for r in regression_list:
        print(f"  {r['name']}")
        print(f"    Baseline: {r['baseline']:.2f} ns")
        print(f"    Current:  {r['current']:.2f} ns")
        print(f"    Change:   +{r['change']:.1f}% (REGRESSION)")
        print()
    print(f"Benchmark gate FAILED (threshold: {threshold}%)")
    sys.exit(1)
elif improvement_list:
    print(f"✅ NO REGRESSIONS")
    print()
    for i in improvement_list:
        print(f"  {i['name']}: {i['change']:.1f}% faster (improvement)")
    print()
    print("Benchmark gate PASSED")
    sys.exit(0)
else:
    print(f"✅ Benchmark gate PASSED")
    print(f"   {len(baseline_map)} benchmarks compared")
    print(f"   All within ±{threshold}%")
    sys.exit(0)
EOF

exit_code=$?

if [ $exit_code -eq 0 ]; then
    echo ""
    echo "✅ Benchmark Gate: PASSED"
else
    echo ""
    echo "❌ Benchmark Gate: FAILED"
fi

exit $exit_code
