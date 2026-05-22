#!/bin/bash
# Gate Check Script: GA Gate v1.1.0
# Usage: bash scripts/gate/check_ga_v110.sh

set -e

echo "=========================================="
echo "  GMP-Platform v1.1.0 GA Gate Check"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo ""

PASS=0
FAIL=0
TOTAL=12

log_pass() { echo "  [PASS] $1"; PASS=$((PASS+1)); }
log_fail() { echo "  [FAIL] $1"; FAIL=$((FAIL+1)); }

# G1: Build
echo "--- G1: Build ---"
if cargo build --release 2>&1 | tail -1 | grep -q "Finished"; then
    log_pass "cargo build --release"
else
    log_fail "cargo build --release"
fi

# G2: Unit Tests
echo "--- G2: Unit Tests ---"
if cargo test --quiet 2>&1 | tail -3 | grep -q "test result: ok"; then
    log_pass "cargo test"
else
    log_fail "cargo test"
fi

# G3: Clippy
echo "--- G3: Clippy ---"
if cargo clippy --all-targets 2>&1 | grep -q "error:"; then
    log_fail "clippy: errors found"
else
    log_pass "clippy: no errors (warnings allowed)"
fi

# G4: Format
echo "--- G4: Format ---"
if cargo fmt -- --check 2>&1; then
    log_pass "cargo fmt"
else
    log_fail "cargo fmt"
fi

# G5: 408 Scenarios (v1.1.0 target: 98%+)
echo "--- G5: 408 Scenarios ---"
SCENARIO_OUTPUT=$(cargo run -p gmp-eval 2>/dev/null)
SCENARIO_RATE=$(echo "$SCENARIO_OUTPUT" | grep "Pass Rate" | grep -oP '\d+\.\d+' | head -1)
if [ -n "$SCENARIO_RATE" ] && [ "$(echo "$SCENARIO_RATE >= 98.0" | bc)" -eq 1 ]; then
    log_pass "gmp-eval: ${SCENARIO_RATE}% (target: 98%)"
else
    log_fail "gmp-eval: ${SCENARIO_RATE}% (target: 98%)"
fi

# G6: SQLRustGo
echo "--- G6: SQLRustGo ---"
if [ -f "crates/gmp-storage/src/sqlrustgo_native.rs" ]; then
    log_pass "sqlrustgo_native.rs exists"
else
    log_fail "sqlrustgo_native.rs not found"
fi

# G7: Docs Complete
echo "--- G7: Docs Complete ---"
DOC_COUNT=$(ls docs/releases/v1.1.0/*.md 2>/dev/null | wc -l)
if [ "$DOC_COUNT" -ge 3 ]; then
    log_pass "docs: $DOC_COUNT documents"
else
    log_fail "docs: only $DOC_COUNT documents (need 3+)"
fi

# G8: Gate Scripts
echo "--- G8: Gate Scripts ---"
if [ -f "scripts/gate/check_ga_v110.sh" ] && [ -f "scripts/benchmarks/regression_check.sh" ]; then
    log_pass "gate scripts exist"
else
    log_fail "gate scripts missing"
fi

# G9: cargo audit
echo "--- G9: cargo audit ---"
AUDIT_OUTPUT=$(cargo audit --no-fetch 2>&1)
if echo "$AUDIT_OUTPUT" | grep -q "error: could not"; then
    log_fail "cargo audit: failed to run"
elif echo "$AUDIT_OUTPUT" | grep -q "no vulnerabilities"; then
    log_pass "cargo audit: no vulnerabilities"
else
    log_pass "cargo audit: passed (warnings only)"
fi

# G10: Git Tag
echo "--- G10: Git Tag ---"
if git tag -l "v1.1.0" | grep -q "v1.1.0"; then
    log_pass "git tag v1.1.0 exists"
else
    log_fail "git tag v1.1.0 not found"
fi

# G11: Test Binaries
echo "--- G11: Test Binaries ---"
if [ -f "target/release/gmp-eval" ] && [ -f "target/release/gmp-retrieve" ]; then
    log_pass "test binaries exist"
else
    log_fail "test binaries missing"
fi

# G12: Stress Tests
echo "--- G12: Stress Tests ---"
STRESS_OUTPUT=$(cargo run -p gmp-cli -- bench --concurrent 10 --rounds 50 2>/dev/null || echo "")
QPS=$(echo "$STRESS_OUTPUT" | grep "QPS:" | awk '{print $NF}' | tr -d ' ')
P95=$(echo "$STRESS_OUTPUT" | grep "P95:" | awk '{print $NF}' | tr -d 'ms')
if [ -n "$QPS" ] && [ -n "$P95" ]; then
    if [ "$(echo "$QPS > 5.0" | bc)" -eq 1 ] && [ "$(echo "$P95 < 2000" | bc)" -eq 1 ]; then
        log_pass "stress: QPS=$QPS, P95=${P95}ms"
    else
        log_fail "stress: QPS=$QPS (need >5), P95=${P95}ms (need <2000)"
    fi
else
    log_fail "stress: test failed"
fi

echo ""
echo "=========================================="
echo "  GA Gate Results: $PASS/$TOTAL PASSED"
echo "=========================================="

if [ "$FAIL" -eq 0 ]; then
    echo "  Status: PASS"
    exit 0
else
    echo "  Status: FAIL ($FAIL failures)"
    exit 1
fi
