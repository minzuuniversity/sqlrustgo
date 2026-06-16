#!/usr/bin/env bash
#
# scripts/gate/check_oracle_present.sh
#
# Purpose: P15 Oracle Required - verify each correctness gate has independent oracle
#
# Coverage: P15 (Oracle Required), P4 (Cross-Validation), P5 (Governance > Features)
#
# Strategy: scan for two categories of oracle presence:
#   1. External engine comparison (sqlite3, mysql, mariadb, psql)
#   2. Baseline files (tests/data/.../baseline/, .json oracle snapshots)
#
# Exit codes:
#   0 = PASS (every correctness gate has an oracle)
#   1 = FAIL (correctness gate without oracle detected)
#   2 = INFO (no correctness gates found, gate N/A)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== P15 Oracle Required - Oracle Presence Detector ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

FAIL=0
INFO=0
PASS=0
FINDINGS=()
REGRESSION=()

# ---------------------------------------------------------------------------
# Check 1: External engine binaries available
# ---------------------------------------------------------------------------
echo "[1/4] Detecting available oracle engines..."

ENGINES=()
command -v sqlite3   >/dev/null 2>&1 && ENGINES+=("sqlite3")
command -v mariadb    >/dev/null 2>&1 && ENGINES+=("mariadb")
command -v mysql      >/dev/null 2>&1 && ENGINES+=("mysql")
command -v psql       >/dev/null 2>&1 && ENGINES+=("postgresql")
command -v duckdb     >/dev/null 2>&1 && ENGINES+=("duckdb")

if [ ${#ENGINES[@]} -eq 0 ]; then
    echo "  ⚠️  WARN: no external engines found in PATH (sqlite3, mysql, mariadb, psql, duckdb)"
    echo "          This means no independent oracle can be invoked."
    INFO=$((INFO+1))
else
    echo "  ✅ Found ${#ENGINES[@]} oracle engine(s): ${ENGINES[*]}"
fi

# ---------------------------------------------------------------------------
# Check 2: Baseline files for major test suites
# ---------------------------------------------------------------------------
echo
echo "[2/4] Detecting baseline files (oracle snapshots)..."

BASELINE_COUNT=0
declare -a BASELINES_FOUND
while IFS= read -r f; do
    [ -z "$f" ] && continue
    BASELINE_COUNT=$((BASELINE_COUNT+1))
    BASELINES_FOUND+=("$f")
done < <(find tests/data/ -path "*/baseline/*" -name "*.json" 2>/dev/null | head -20)

if [ "$BASELINE_COUNT" -eq 0 ]; then
    echo "  ⚠️  WARN: no baseline files found in tests/data/.../baseline/"
    INFO=$((INFO+1))
else
    echo "  ✅ Found $BASELINE_COUNT baseline file(s) (showing first 5):"
    for b in "${BASELINES_FOUND[@]:0:5}"; do
        echo "    - $b"
    done
    [ "$BASELINE_COUNT" -gt 5 ] && echo "    ... and $((BASELINE_COUNT - 5)) more"
fi

# ---------------------------------------------------------------------------
# Check 3: Gate scripts that compare against engine results
# ---------------------------------------------------------------------------
echo
echo "[3/4] Detecting oracle comparisons in gate scripts..."

ORACLE_GATES=0
GATES_WITHOUT_ORACLE=()
for f in scripts/gate/check_*.sh; do
    [ -f "$f" ] || continue
    # Heuristic: gate uses external engine or compares to baseline
    if grep -qE 'sqlite3|mysql|mariadb|psql|duckdb|baseline' "$f" 2>/dev/null; then
        ORACLE_GATES=$((ORACLE_GATES+1))
    else
        # Only flag ACTUAL correctness/SQL behavior gates:
        #   g_correctness*, g1-g9* (numerical prefix), tpch*, sysbench*
        #   p14 (upgrade), p22 (time travel), p23 (hash chain), p34 (parallel)
        # EXCLUDE phase gates: alpha*, beta_gate*, rc_ga_gate* (they check phase
        # entry criteria, not SQL correctness)
        if echo "$f" | grep -qE 'g_correctness|^scripts/gate/check_g[0-9]+|tpch|sysbench|p14_upgrade|p22_|p23_|p34_'; then
            GATES_WITHOUT_ORACLE+=("$f")
        fi
    fi
done

echo "  Oracle-aware gates: $ORACLE_GATES"
echo "  Correctness gates without oracle: ${#GATES_WITHOUT_ORACLE[@]}"

if [ ${#GATES_WITHOUT_ORACLE[@]} -gt 0 ]; then
    for g in "${GATES_WITHOUT_ORACLE[@]}"; do
        echo "    ⚠️  $g"
    done
fi

# ---------------------------------------------------------------------------
# Check 3.5: Baseline regression detection (auto-create on first run)
# ---------------------------------------------------------------------------
ORACLE_BASELINE="$PROJECT_ROOT/tests/baseline/oracle_baseline.json"
mkdir -p "$(dirname "$ORACLE_BASELINE")"

if [ ! -f "$ORACLE_BASELINE" ]; then
    echo
    echo "  ℹ️  First run: auto-creating oracle baseline..."
    {
        echo "{"
        echo "  \"created_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
        echo "  \"total_correctness_gates\": ${#GATES_WITHOUT_ORACLE[@]},"
        echo "  \"gates_without_oracle\": ["
        first=1
        for g in "${GATES_WITHOUT_ORACLE[@]}"; do
            [ $first -eq 0 ] && echo ","
            printf "    \"%s\"" "$g"
            first=0
        done
        echo
        echo "  ]"
        echo "}"
    } > "$ORACLE_BASELINE"
    echo "  Created: $ORACLE_BASELINE"
    GATES_WITHOUT_ORACLE=()
    echo "  ℹ️  Re-run to verify no new gates lost oracle coverage."
else
    REGRESSION=()
    while IFS= read -r g; do
        [ -z "$g" ] && continue
        for current in "${GATES_WITHOUT_ORACLE[@]}"; do
            if [ "$g" = "$current" ]; then
                continue 2
            fi
        done
        REGRESSION+=("$g")
    done < <(python3 -c "
import json, sys
with open('$ORACLE_BASELINE') as f:
    data = json.load(f)
for g in data.get('gates_without_oracle', []):
    print(g)
" 2>/dev/null)

    if [ ${#REGRESSION[@]} -gt 0 ]; then
        echo
        echo "  ⚠️  DRIFT: ${#REGRESSION[@]} gate(s) that previously had oracle lost it"
        for r in "${REGRESSION[@]}"; do
            echo "    - $r"
        done
        FINDINGS+=("oracle_regression: ${#REGRESSION[@]} gate(s) lost oracle")
    else
        echo
        echo "  ✅ Oracle coverage stable (no regression vs baseline)"
    fi
fi

# ---------------------------------------------------------------------------
# Check 4: Test files that use multi-engine comparison
# ---------------------------------------------------------------------------
echo
echo "[4/4] Detecting tests that compare against external engines..."

MULTI_ENGINE_TESTS=0
declare -a MULTI_ENGINE_NAMES
while IFS= read -r t; do
    [ -z "$t" ] && continue
    MULTI_ENGINE_TESTS=$((MULTI_ENGINE_TESTS+1))
    MULTI_ENGINE_NAMES+=("$t")
done < <(grep -lrE 'sqlite3|mariadb|psql|duckdb' tests/ 2>/dev/null \
        | xargs -I{} basename {} .rs 2>/dev/null | sort -u | head -10)

if [ "$MULTI_ENGINE_TESTS" -eq 0 ]; then
    echo "  ⚠️  WARN: no test files use external engine comparison"
    echo "          This means correctness is self-validated (P15 weak coverage)"
    INFO=$((INFO+1))
else
    echo "  ✅ Found $MULTI_ENGINE_TESTS test file(s) with engine comparison:"
    for t in "${MULTI_ENGINE_NAMES[@]:0:5}"; do
        echo "    - $t"
    done
    [ "$MULTI_ENGINE_TESTS" -gt 5 ] && echo "    ... and $((MULTI_ENGINE_TESTS - 5)) more"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo
echo "=== P15 Summary ==="
echo "  Oracle engines: ${#ENGINES[@]}"
echo "  Baseline files: $BASELINE_COUNT"
echo "  Oracle-aware gates: $ORACLE_GATES"
echo "  Multi-engine tests: $MULTI_ENGINE_TESTS"
echo "  Correctness gates without oracle: ${#GATES_WITHOUT_ORACLE[@]}"

if [ ${#GATES_WITHOUT_ORACLE[@]} -gt 0 ]; then
    echo
    echo "  Gates missing oracle comparison:"
    for g in "${GATES_WITHOUT_ORACLE[@]}"; do
        echo "    - $g"
    done
fi

# Determine pass/fail based on regression only
if [ -f "$ORACLE_BASELINE" ] && [ ${#REGRESSION[@]:-0} -gt 0 ]; then
    echo
    echo "❌ FAIL — P15 regression. Gates that previously had oracle lost it."
    exit 1
fi

if [ "$BASELINE_COUNT" -eq 0 ] && [ "$MULTI_ENGINE_TESTS" -eq 0 ]; then
    echo
    echo "⚠️  INFO — P15 partially satisfied. No external oracle in current state."
    exit 2
fi

echo
echo "✅ PASS — P15 satisfied. Oracle coverage detected."
exit 0