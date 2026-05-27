#!/usr/bin/env bash
# check_alpha_v350.sh - Alpha Gate Check for v3.5.0
# Executes all A-Gate checks per gate_spec_v350.md

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== v3.5.0 Alpha Gate Check ==="
echo "Date: $(date)"
echo "Branch: $(git rev-parse --abbrev-ref HEAD)"
echo "Commit: $(git rev-parse HEAD)"
echo ""

FAILED=0

run_check() {
    local name="$1"
    local cmd="$2"
    echo -n "[A] $name ... "
    if bash -c "$cmd" > /dev/null 2>&1; then
        echo "✅ PASS"
    else
        echo "❌ FAIL"
        FAILED=1
    fi
}

run_check_report() {
    local name="$1"
    local cmd="$2"
    echo -n "[A] $name ... "
    if bash -c "$cmd"; then
        echo "✅ PASS"
    else
        echo "❌ FAIL"
        FAILED=1
    fi
}

cd "$PROJECT_ROOT"

echo "=== A1: Build ==="
run_check "Build" "cargo build --release --workspace"

echo ""
echo "=== A2: Unit Tests ==="
run_check "Unit Tests" "cargo test --lib"

echo ""
echo "=== A3: Clippy ==="
run_check "Clippy" "cargo clippy --all-features -- -D warnings"

echo ""
echo "=== A4: Format ==="
run_check "Format" "cargo fmt --all -- --check"

echo ""
echo "=== A5: Coverage ==="
bash "$SCRIPT_DIR/check_coverage.sh" || FAILED=1

echo ""
echo "=== TI: Trust Infrastructure ==="
run_check_report "TI-1 perf-baseline" "[ -d crates/perf-baseline/src ]"
run_check_report "TI-2 crash-sim" "[ -d crates/crash-sim/src ]"
run_check_report "TI-3 wal-verification" "[ -d crates/wal-verification/src ]"
run_check_report "TI-4 compliance-engine" "[ -d crates/compliance-engine/src ]"
run_check_report "TI-5 evidence-engine" "[ -d crates/evidence-engine/src ]"
run_check_report "TI-6 provenance-graph" "[ -d crates/provenance-graph/src ]"
run_check_report "TI-7 gmp-api" "[ -d crates/gmp-api/src ]"
run_check_report "TI-8 gmp" "[ -d crates/gmp/src ]"
run_check_report "TI-9 gmp-retrieval" "[ -d crates/gmp-retrieval/src ]"

echo ""
echo "=== SSOT: Specification Docs ==="
run_check_report "SSOT-1 gate_spec_v350" "[ -f docs/governance/gate_spec_v350.md ]"
run_check_report "SSOT-2 test_plan" "[ -f docs/releases/v3.5.0/TEST_PLAN.md ]"

echo ""
echo "=== v3.5.0 New Crate Tests ==="
run_check "compliance-engine tests" "cargo test -p sqlrustgo-compliance-engine --lib"
run_check "gmp-api tests" "cargo test -p sqlrustgo-gmp-api --lib"
run_check "gmp tests" "cargo test -p sqlrustgo-gmp --lib"
run_check "gmp-retrieval tests" "cargo test -p sqlrustgo-gmp-retrieval --lib"

echo ""
echo "=== Alpha Gate Result: $([ $FAILED -eq 0 ] && echo '✅ PASSED' || echo '❌ FAILED') ==="
exit $FAILED
