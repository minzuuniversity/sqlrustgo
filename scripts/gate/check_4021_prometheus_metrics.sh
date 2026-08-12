#!/usr/bin/env bash
# check_4021_prometheus_metrics.sh — V312-26 Issue #4021 Prometheus /metrics Gate
#
# Verifies the Prometheus /metrics endpoint infrastructure for #4021:
# 1. Code surface: metrics_endpoint.rs (mysql-server) +
#    prometheus.rs (telemetry renderer) + metrics_aggregator.rs (common)
# 2. mysql-server depends on sqlrustgo-telemetry
# 3. unit tests pass: telemetry prometheus_test (16+) + mysql-server metrics_endpoint (9)
# 4. e2e smoke: spin up mysql-server with --metrics-port and curl /metrics,
#    asserting HTTP 200 + Prometheus text format 0.0.4 + required metric names
# 5. Evidence doc + snapshot exist
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

# 2. Crate implementation
echo ""
echo "--- Renderer Implementation ---"
check "telemetry/src/prometheus.rs has PrometheusRenderer" "grep -q 'impl PrometheusRenderer' crates/telemetry/src/prometheus.rs"
check "telemetry crate referenced from mysql-server" "grep -q 'sqlrustgo-telemetry' crates/mysql-server/Cargo.toml"

# 3. Tests: telemetry prometheus_test (integration) + mysql-server metrics_endpoint (unit)
echo ""
echo "--- Unit Tests ---"
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

me_out=$(timeout 180 cargo test -p sqlrustgo-mysql-server --lib metrics_endpoint --quiet 2>&1 | tail -3)
if echo "$me_out" | grep -q "test result: ok"; then
    me_summary=$(echo "$me_out" | grep "test result" | tail -1)
    echo "  [PASS] mysql-server metrics_endpoint unit tests: ${me_summary}"
    PASS=$((PASS+1))
else
    echo "  [FAIL] mysql-server metrics_endpoint unit tests failed"
    echo "         last lines: $me_out"
    FAIL=$((FAIL+1))
fi

# 4. e2e smoke: start mysql-server with --metrics-port and curl /metrics
echo ""
echo "--- E2E /metrics Scrape ---"
# Build binary (skip if exists)
if [ ! -x "target/debug/sqlrustgo-mysql-server" ]; then
    echo "  [INFO] Building sqlrustgo-mysql-server debug binary (one-time)..."
    cargo build -p sqlrustgo-mysql-server --bin sqlrustgo-mysql-server --quiet 2>&1 | tail -3
fi

# Pick two free ports
MYSQL_PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')
METRICS_PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')
DATA_DIR=$(mktemp -d /tmp/sqlrustgo-v4021-gate-data.XXXXXX)
EVIDENCE_DIR="docs/releases/v3.12.0/evidence/issue-4021"
mkdir -p "$EVIDENCE_DIR"
SNAPSHOT="$EVIDENCE_DIR/metrics_endpoint.snapshot"

# Launch
./target/debug/sqlrustgo-mysql-server serve \
    --host 127.0.0.1 --port "$MYSQL_PORT" \
    --data-dir "$DATA_DIR" \
    --max-connections 4 --server-threads 2 \
    --metrics-port "$METRICS_PORT" \
    > /tmp/v4021_gate_server.out 2> /tmp/v4021_gate_server.err &
SERVER_PID=$!
# Wait for listener
for _ in 1 2 3 4 5 6 7 8 9 10; do
    sleep 0.5
    if curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:${METRICS_PORT}/metrics" 2>/dev/null | grep -q "^200$"; then
        break
    fi
done

# Issue a query so the wire-protocol counters reflect real activity
if command -v mysql >/dev/null 2>&1; then
    mysql -h 127.0.0.1 -P "$MYSQL_PORT" -u root --ssl-mode=DISABLED -B -e "SELECT 1;" >/dev/null 2>&1 || true
fi

# Scrape /metrics (use -i to include headers in a separate capture for Content-Type check)
http_code=$(curl -s -o "$SNAPSHOT" -w "%{http_code}" "http://127.0.0.1:${METRICS_PORT}/metrics")
curl_headers=$(curl -s -D - -o /dev/null "http://127.0.0.1:${METRICS_PORT}/metrics" 2>/dev/null || true)
body_size=$(wc -c < "$SNAPSHOT" 2>/dev/null || echo 0)

if [ "$http_code" = "200" ]; then
    echo "  [PASS] GET /metrics → HTTP 200 (${body_size} bytes, saved to ${SNAPSHOT})"
    PASS=$((PASS+1))
else
    echo "  [FAIL] GET /metrics → HTTP ${http_code}"
    FAIL=$((FAIL+1))
fi

# Assert Prometheus text format 0.0.4 (headers from the second curl, body from $SNAPSHOT)
check "response Content-Type is text/plain; version=0.0.4" "echo '$curl_headers' | grep -qi 'Content-Type: text/plain; version=0.0.4'"
check "snapshot contains # HELP" "grep -q '# HELP' $SNAPSHOT"
check "snapshot contains # TYPE" "grep -q '# TYPE' $SNAPSHOT"
check "snapshot has sqlrustgo_active_connections" "grep -q '^sqlrustgo_active_connections' $SNAPSHOT"
check "snapshot has sqlrustgo_queries_total" "grep -q '^sqlrustgo_queries_total' $SNAPSHOT"

# Verify /health
check "GET /health returns 200" "curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:${METRICS_PORT}/health | grep -q '^200$'"
check "GET /unknown returns 404" "curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:${METRICS_PORT}/unknown | grep -q '^404$'"

# Cleanup
kill "$SERVER_PID" 2>/dev/null || true
sleep 0.3
kill -9 "$SERVER_PID" 2>/dev/null || true
rm -rf "$DATA_DIR"

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
if [ -s "$SNAPSHOT" ]; then
    echo "  [PASS] metrics_endpoint.snapshot exists (${body_size} bytes)"
    PASS=$((PASS+1))
else
    echo "  [FAIL] metrics_endpoint.snapshot missing or empty"
    FAIL=$((FAIL+1))
fi

echo ""
echo "=== #4021 Prometheus /metrics Gate Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "✅ #4021 gate PASSED (Prometheus /metrics endpoint code + unit tests + live e2e scrape verified)"
    exit 0
else
    echo "❌ #4021 gate FAILED ($FAIL blocker(s))"
    exit 1
fi
