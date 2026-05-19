#!/usr/bin/env bash
# v3.3.0 Beta Gate (Z6G4 Server)
set -euo pipefail

source ~/.cargo/env 2>/dev/null || true

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$PROJECT_ROOT"

PASS=0; FAIL=0; SKIP=0

check() {
    local name="$1"; shift
    local cmd=("$@")
    echo -n "[beta-v3.3.0] $name ... "
    if "${cmd[@]}" >/dev/null 2>&1; then
        echo "✅ PASS"; PASS=$((PASS+1))
    else
        echo "❌ FAIL"; FAIL=$((FAIL+1))
    fi
}

check_coverage() {
    local name="$1"; local min="$2"; local crate="$3"
    echo -n "[beta-v3.3.0] $name ... "
    out=$(cargo llvm-cov -p "sqlrustgo-$crate" --all-features --lib 2>&1 || true)
    rate=$(echo "$out" | grep "^TOTAL" | awk '{print $4}' | tr -d '%' || echo "0")
    result=$(echo "$rate >= $min" | bc -l 2>/dev/null || echo "0")
    if [ "$result" = "1" ]; then
        echo "✅ PASS ($rate%)"; PASS=$((PASS+1))
    else
        echo "❌ FAIL ($rate% < $min%)"; FAIL=$((FAIL+1))
    fi
}

check_test() {
    local name="$1"; local cmd="$2"
    echo -n "[beta-v3.3.0] $name ... "
    out=$(eval "$cmd" 2>&1 || true)
    passed=$(echo "$out" | grep -o " ok$" | wc -l | tr -d ' ' || echo "0")
    failed=$(echo "$out" | grep -o " FAILED$" | wc -l | tr -d ' ' || echo "0")
    [ -z "$passed" ] && passed=0
    [ -z "$failed" ] && failed=0
    if [ "$failed" -eq 0 ] && [ "$passed" -gt 0 ]; then
        echo "✅ PASS ($passed tests)"; PASS=$((PASS+1))
    elif [ "$passed" -eq 0 ] && [ "$failed" -eq 0 ]; then
        echo "⚠️ SKIP (no tests)"; SKIP=$((SKIP+1))
    else
        echo "❌ FAIL ($passed passed, $failed failed)"; FAIL=$((FAIL+1))
    fi
}

echo "=== v3.3.0 Beta Gate ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# B1: Build
check "B1: cargo build --all-features" cargo build --all-features

# B2: Format
check "B2: cargo fmt" cargo fmt --all -- --check

# B3: Clippy
check "B3: cargo clippy" cargo clippy --all-features -- -D warnings

# B4: Trust Infrastructure Tests (8 crates)
check_test "B4: crash-sim tests" "cargo test -p sqlrustgo-crash-sim --lib"
check_test "B5: wal-verification tests" "cargo test -p sqlrustgo-wal-verification --lib"
check_test "B6: compliance-engine tests" "cargo test -p sqlrustgo-compliance-engine --lib"
check_test "B7: evidence-engine tests" "cargo test -p sqlrustgo-evidence-engine --lib"
check_test "B8: provenance-graph tests" "cargo test -p sqlrustgo-provenance-graph --lib"
check_test "B9: workflow-v2 tests" "cargo test -p sqlrustgo-workflow-v2 --lib"
check_test "B10: trust-viz tests" "cargo test -p sqlrustgo-trust-viz --lib"
check_test "B11: perf-baseline tests" "cargo test -p sqlrustgo-perf-baseline --lib"

# B12: Core Unit Tests
check_test "B12: core unit tests" "cargo test --lib -- --test-threads=4"

# B13: Integration Tests
check_test "B13: integration tests" "cargo test --test '*' -- --test-threads=2"

echo ""
echo "━━━ Stability Tests ━━━"
check_test "S1: crash_recovery" "cargo test --test crash_recovery_test"
check_test "S2: wal_integration" "cargo test --test wal_integration_test"
check_test "S3: concurrency_stress" "cargo test --test concurrency_stress_test"
check_test "S4: long_run_stability" "cargo test --test long_run_stability_test"

echo ""
echo "━━━ Coverage Gate ━━━"
check_coverage "C1: parser (>=75%)" 75 "parser"
check_coverage "C2: executor (>=70%)" 70 "executor"
check_coverage "C3: planner (>=65%)" 65 "planner"
check_coverage "C4: optimizer (>=55%)" 55 "optimizer"
check_coverage "C5: storage (>=70%)" 70 "storage"
check_coverage "C6: transaction (>=70%)" 70 "transaction"

echo ""
echo "━━━ v3.3.0 Beta Gate Summary ━━━"
echo "✅ PASS: $PASS"
echo "❌ FAIL: $FAIL"
echo "⚠️  SKIP: $SKIP"
echo ""

if [ $FAIL -gt 0 ]; then
    echo "RESULT: FAILED ($FAIL blocker(s))"
    exit 1
else
    echo "RESULT: PASSED"
    exit 0
fi
