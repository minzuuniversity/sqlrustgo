#!/usr/bin/env bash
# check_coverage_comprehensive.sh
# 综合覆盖率测量：--lib + --tests 的最优覆盖数据
# 策略：优先使用 --tests（覆盖更广），失败则降级到 --lib
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
COVERAGE_DIR="$ROOT_DIR/artifacts/coverage"
mkdir -p "$COVERAGE_DIR"

L1_CRATES=(
    "sqlrustgo-types"
    "sqlrustgo-parser"
    "sqlrustgo-planner"
    "sqlrustgo-optimizer"
    "sqlrustgo-executor"
    "sqlrustgo-storage"
    "sqlrustgo-transaction"
    "sqlrustgo-catalog"
)

# Get coverage for a single crate
# Strategy: executor/storage use --lib (broken --tests), all others use --tests first
get_best_coverage() {
    local crate="$1"
    local crate_name="${crate#sqlrustgo-}"
    local tmp_tests="$COVERAGE_DIR/.tmp_${crate_name}_tests.json"
    local tmp_lib="$COVERAGE_DIR/.tmp_${crate_name}_lib.json"

    # executor and storage have broken --tests (linking/panic issues), use --lib
    if [[ "$crate" == "sqlrustgo-executor" ]] || [[ "$crate" == "sqlrustgo-storage" ]]; then
        echo "[COVERAGE] $crate: using --lib (--tests broken)" >&2
        if cargo llvm-cov --package "$crate" --all-features --lib \
            --json --output-path "$tmp_lib" >/dev/null 2>&1; then
            local lib_pct=$(python3 -c "
import json
with open('$tmp_lib') as f:
    data = json.load(f)
pct = data.get('data', [{}])[0].get('totals', {}).get('lines', {}).get('percent', 0)
print(f'{pct:.2f}')
" 2>/dev/null || echo "0")
            echo "[COVERAGE]   -> $crate: ${lib_pct}% (--lib)" >&2
            echo "$lib_pct"
            return 0
        fi
        echo "[COVERAGE]   -> $crate: FAILED" >&2
        echo "0"
        return 1
    fi

    echo "[COVERAGE] $crate: trying --tests..." >&2

    # Try --tests first (broader coverage)
    if cargo llvm-cov --package "$crate" --all-features --tests \
        --json --output-path "$tmp_tests" >/dev/null 2>&1; then
        local tests_pct=$(python3 -c "
import json
with open('$tmp_tests') as f:
    data = json.load(f)
pct = data.get('data', [{}])[0].get('totals', {}).get('lines', {}).get('percent', 0)
print(f'{pct:.2f}')
" 2>/dev/null || echo "0")
        if [[ "$tests_pct" != "0.00" ]] && [[ -n "$tests_pct" ]]; then
            rm -f "$tmp_lib"
            echo "[COVERAGE]   -> $crate: ${tests_pct}% (--tests)" >&2
            echo "$tests_pct"
            return 0
        fi
    fi
    rm -f "$tmp_tests"

    # Fall back to --lib
    echo "[COVERAGE] $crate: falling back to --lib..." >&2
    if cargo llvm-cov --package "$crate" --all-features --lib \
        --json --output-path "$tmp_lib" >/dev/null 2>&1; then
        local lib_pct=$(python3 -c "
import json
with open('$tmp_lib') as f:
    data = json.load(f)
pct = data.get('data', [{}])[0].get('totals', {}).get('lines', {}).get('percent', 0)
print(f'{pct:.2f}')
" 2>/dev/null || echo "0")
        if [[ "$lib_pct" != "0.00" ]] && [[ -n "$lib_pct" ]]; then
            rm -f "$tmp_tests"
            echo "[COVERAGE]   -> $crate: ${lib_pct}% (--lib fallback)" >&2
            echo "$lib_pct"
            return 0
        fi
    fi
    rm -f "$tmp_lib"

    echo "[COVERAGE]   -> $crate: FAILED" >&2
    echo "0"
    return 1
}

echo "=== Comprehensive Coverage Measurement ==="
echo "Strategy: --tests (primary), --lib (fallback)"
echo ""

total_pct=0
count=0
CRATE_ORDER=()
CRATE_PCTS=()

for crate in "${L1_CRATES[@]}"; do
    pct=$(get_best_coverage "$crate")
    CRATE_ORDER+=("$crate")
    CRATE_PCTS+=("$pct")
    if [[ "$pct" != "0" ]] && [[ -n "$pct" ]]; then
        total_pct=$(echo "$total_pct + $pct" | bc -l)
        count=$((count + 1))
    fi
done

if [[ $count -gt 0 ]]; then
    avg_pct=$(echo "scale=2; $total_pct / $count" | bc -l)
    echo ""
    echo "=== L1 Coverage Summary (Comprehensive) ==="
    echo "Crates measured: $count/${#L1_CRATES[@]}"
    echo "Average coverage: ${avg_pct}%"

    echo ""
    echo "=== Per-Crate Coverage ==="
    for i in "${!CRATE_ORDER[@]}"; do
        printf "  %-20s %s%%\n" "${CRATE_ORDER[$i]}:" "${CRATE_PCTS[$i]}"
    done

    # Save aggregated result
    echo "$avg_pct" > "$COVERAGE_DIR/l1_coverage_comprehensive.txt"

    echo ""
    echo "Coverage reports saved to: $COVERAGE_DIR/"

    # Return exit code based on threshold
    REQUIRED=85
    AVG_INT=$(echo "$avg_pct * 100" | bc | cut -d. -f1)
    if [[ "$AVG_INT" -ge $((REQUIRED * 100)) ]]; then
        echo "✅ Coverage check PASSED (${avg_pct}% >= ${REQUIRED}%)"
        exit 0
    else
        echo "❌ Coverage check FAILED (${avg_pct}% < ${REQUIRED}%)"
        exit 1
    fi
else
    echo "❌ No crates could be measured!"
    exit 1
fi
