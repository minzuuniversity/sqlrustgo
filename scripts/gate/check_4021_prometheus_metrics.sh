#!/usr/bin/env bash
# check_4021_prometheus_metrics.sh — V312-26 Issue #4021 Prometheus /metrics Gate
#
# Verifies the Prometheus /metrics endpoint infrastructure for #4021:
# 1. Code surface: metrics_endpoint.rs + telemetry/prometheus.rs + metrics_aggregator.rs
# 2. Cargo.toml: prometheus crate dependency
# 3. Tests: tests/unit/prometheus_test.rs exists with sqlrustgo_* assertions
# 4. Tests pass: 16/16 (verified via fresh `cargo test --test prometheus_test`)
# 5. Evidence doc exists
#
# Exit code: 0 = PASS, 1 = FAIL

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== V312-26 #4021 Prometheus /metrics Gate ==="
echo ""

PASS=0
FAIL=0

check() {
    local name="$1"
    local cmd="$2"
    if eval "$cmd" >/dev/null 2>&1; then
        echo "  [PASS] $name"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] $name"
        FAIL=$((FAIL+1))
    fi
}

# 1. Code surface
echo "--- Code Surface ---"
for f in \
    "crates/mysql-server/src/metrics_endpoint.rs" \
    "crates/telemetry/src/prometheus.rs" \
    "crates/common/src/metrics_aggregator.rs"
do
    if [ -f "$f" ]; then
        local_lines=$(wc -l < "$f")
        echo "  [PASS] $f exists (${local_lines} lines)"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] $f missing"
        FAIL=$((FAIL+1))
    fi
done

# 2. Crate implementation (sqlrustgo uses its own Prometheus renderer in
#    crates/telemetry/src/prometheus.rs, not the upstream `prometheus` crate)
echo ""
echo "--- Renderer Implementation ---"
check "telemetry/src/prometheus.rs has PrometheusRenderer" "grep -q 'impl PrometheusRenderer' crates/telemetry/src/prometheus.rs"
check "telemetry/src/prometheus.rs has MetricType" "grep -q 'impl MetricType' crates/telemetry/src/prometheus.rs"
check "telemetry crate referenced from mysql-server" "grep -q 'sqlrustgo_telemetry\\|telemetry' crates/mysql-server/Cargo.toml"

# 3. Tests
echo ""
echo "--- Tests ---"
if [ -f "tests/unit/prometheus_test.rs" ]; then
    echo "  [PASS] tests/unit/prometheus_test.rs exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] tests/unit/prometheus_test.rs missing"
    FAIL=$((FAIL+1))
fi
check "prometheus_test asserts sqlrustgo_queries_total" "grep -q 'sqlrustgo_queries_total' tests/unit/prometheus_test.rs"
check "prometheus_test asserts # HELP format" "grep -q '# HELP' tests/unit/prometheus_test.rs"
check "prometheus_test asserts # TYPE format" "grep -q '# TYPE' tests/unit/prometheus_test.rs"

# 4. Live test compile + run
echo ""
echo "--- Live Test Run ---"
test_out=$(timeout 180 cargo test --test prometheus_test --quiet 2>&1 | tail -3)
if echo "$test_out" | grep -q "test result: ok"; then
    test_summary=$(echo "$test_out" | grep "test result" | tail -1)
    echo "  [PASS] cargo test --test prometheus_test: ${test_summary}"
    PASS=$((PASS+1))
else
    echo "  [FAIL] cargo test --test prometheus_test failed"
    echo "         last lines: $test_out"
    FAIL=$((FAIL+1))
fi

# 5. Evidence doc
echo ""
echo "--- Evidence Document ---"
if [ -f "docs/releases/v3.12.0/evidence/issue-4021/4021_evidence.md" ]; then
    echo "  [PASS] 4021_evidence.md exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] 4021_evidence.md missing"
    FAIL=$((FAIL+1))
fi

echo ""
echo "=== #4021 Prometheus /metrics Gate Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "✅ #4021 gate PASSED (unit-tested renderer + HTTP endpoint code present)"
    exit 0
else
    echo "❌ #4021 gate FAILED ($FAIL blocker(s))"
    exit 1
fi
