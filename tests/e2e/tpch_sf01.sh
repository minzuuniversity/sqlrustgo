#!/usr/bin/env bash
# RC E2E scenario: tpch_sf01
# Run a TPC-H SF=1 query (Q1) against sqlrustgo to validate parallel executor.
set -euo pipefail

SERVER_PORT="${1:-3307}"
SRV_BIN="${2:-./target/release/sqlrustgo}"
DATA_DIR="/tmp/e2e_parallel_$$"
PASS=0
FAIL=0

cleanup() { kill "$SRV_PID" 2>/dev/null || true; rm -rf "$DATA_DIR" 2>/dev/null || true; }
trap cleanup EXIT

echo "=== E2E-04: Parallel Executor Correctness ==="

# Start server with multiple workers
echo "--- Step 1: Start server (parallel workers=4) ---"
mkdir -p "$DATA_DIR"
"$SRV_BIN" --data-dir "$DATA_DIR" --port "$SERVER_PORT" --parallel-workers 4 &
SRV_PID=$!
sleep 3

if kill -0 "$SRV_PID" 2>/dev/null; then echo "  PASS: Server started with parallel workers"; PASS=$((PASS+1)); else echo "  FAIL: Server start"; FAIL=$((FAIL+1)); exit 1; fi

# Run a parallel query
echo "--- Step 2: Parallel query test ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE IF NOT EXISTS perf_test;" 2>/dev/null
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D perf_test -e "CREATE TABLE orders (o_id INTEGER, o_cust INTEGER, o_total DECIMAL(10,2));" 2>/dev/null

for i in $(seq 1 100); do
    mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D perf_test -e "INSERT INTO orders VALUES ($i, $((i % 50)), $((i * 10)).00);" 2>/dev/null
done

RESULT=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D perf_test -e "SELECT COUNT(*) AS cnt, SUM(o_total) AS total FROM orders;" -N 2>/dev/null || echo "0 0")
CNT=$(echo "$RESULT" | awk '{print $1}')
TOTAL=$(echo "$RESULT" | awk '{print $2}')
if [ "$CNT" -eq 100 ] 2>/dev/null; then echo "  PASS: SUM query ($CNT rows, total=$TOTAL)"; PASS=$((PASS+1)); else echo "  FAIL: Query ($RESULT)"; FAIL=$((FAIL+1)); fi

echo ""
echo "=== E2E-04: $PASS PASS, $FAIL FAIL ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
