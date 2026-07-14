#!/usr/bin/env bash
# scripts/coverage/llvm_cov_baseline.sh
#
# Generate per-crate `*-lib.json` coverage data for the v3.10.0 baseline.
# Output: docs/releases/v3.10.0/coverage-baseline/<crate>-lib.json
#         docs/releases/v3.10.0/coverage-baseline/summary.json
#
# RC Gate R6 (check_rc_gate_v3.10.0.sh, lines 195-215) requires each
# crate's <crate>-lib.json to have data[0].summary.percent_covered >= 80.
#
# Usage:
#   bash scripts/coverage/llvm_cov_baseline.sh
#   CRATE_TARGET=80 bash scripts/coverage/llvm_cov_baseline.sh  # custom target
#
# Exits 0 if all crates meet target; non-zero otherwise.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

CRATE_TARGET="${CRATE_TARGET:-80}"
OUT_DIR="$REPO_ROOT/docs/releases/v3.10.0/coverage-baseline"
mkdir -p "$OUT_DIR"

# Ensure cargo-llvm-cov is on PATH
if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo-llvm-cov" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo "❌ cargo-llvm-cov not installed. Run: cargo install cargo-llvm-cov"
        exit 2
    fi
fi

# Discover workspace members
WORKSPACE_CRATES=$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
    | python3 -c "
import json, sys
d = json.load(sys.stdin)
for pkg in d.get('packages', []):
    name = pkg.get('name')
    # Skip the root virtual package if any
    if name and not name.startswith('sqlrustgo-') and name != 'sqlrustgo-cli' and name != 'sqlrustgo_sqllogictest':
        # Filter to actual lib crates (have a lib.rs)
        if any(t.get('kind') == ['lib'] for t in pkg.get('targets', [])):
            print(name)
" 2>/dev/null | sort -u)

if [ -z "$WORKSPACE_CRATES" ]; then
    echo "❌ No workspace crates discovered. Run from repo root."
    exit 2
fi

echo "=== llvm-cov baseline generation ==="
echo "Output dir: $OUT_DIR"
echo "Per-crate target: $CRATE_TARGET%"
echo "Crates to process:"
echo "$WORKSPACE_CRATES" | sed 's/^/  - /'
echo ""

SUMMARY_JSON="$OUT_DIR/summary.json"
echo "[]" > "$SUMMARY_JSON"

PASS=0
FAIL=0

for crate in $WORKSPACE_CRATES; do
    echo "--- [$crate] running cargo llvm-cov --lib --json ---"
    json_path="$OUT_DIR/${crate}-lib.json"
    if cargo llvm-cov -p "$crate" --lib --json --output-path "$json_path" 2>/dev/null; then
        pct=$(python3 -c "
import json, sys
try:
    d = json.load(open('$json_path'))
    s = d.get('data', [{}])[0].get('summary', {})
    print(round(s.get('percent_covered', 0), 2))
except Exception:
    print('?')
" 2>/dev/null)
        if [ "$pct" != "?" ] && [ "${pct%.*}" -ge "$CRATE_TARGET" ]; then
            echo "  ✅ $crate: ${pct}% (≥${CRATE_TARGET}%)"
            PASS=$((PASS + 1))
        else
            echo "  ❌ $crate: ${pct}% (<${CRATE_TARGET}%)"
            FAIL=$((FAIL + 1))
        fi
        # Update summary.json
        python3 - <<PY
import json
agg = json.load(open("$SUMMARY_JSON"))
agg.append({"crate": "$crate", "percent_covered": float("$pct") if "$pct" != "?" else 0.0})
json.dump(agg, open("$SUMMARY_JSON", "w"), indent=2)
PY
    else
        echo "  ❌ $crate: cargo llvm-cov failed"
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "=== Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo "Output: $OUT_DIR"
echo "Aggregated: $SUMMARY_JSON"

if [ "$FAIL" -gt 0 ]; then
    echo "❌ Not all crates meet ${CRATE_TARGET}% target."
    exit 1
fi

echo "✅ All crates meet ${CRATE_TARGET}% target."
exit 0
