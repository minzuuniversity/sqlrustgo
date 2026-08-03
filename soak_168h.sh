#!/bin/bash
# 168h SOAK Test for v3.11.0 RC
# Target: 40-60% CPU utilization with long connections
set -euo pipefail

HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-3306}"
THREADS="${THREADS:-24}"           # Increased from 8 for higher throughput
QPS="${QPS:-0}"                     # 0 = unlimited, let CPU naturally limit
DURATION="${DURATION:-604800}"      # 168 hours
OLTP_RATIO="${OLTP_RATIO:-0.4}"   # 40% OLTP, 60% OLAP (mixed workload)
RESULTS_DIR="${HOME}/sqlrustgo-soak/v311-soak-$(date '+%Y%m%d_%H%M%S')"

BINARY="$(pwd)/target/release/sqlrustgo-mysql-server"
HYBRID_SOAK="$(pwd)/target/release/examples/hybrid_soak"

mkdir -p "$RESULTS_DIR"

echo "=== 168h SOAK Test Started ==="
echo "Time: $(date)"
echo "Results: $RESULTS_DIR"
echo "Host: $HOST:$PORT"
echo "Threads: $THREADS"
echo "OLTP Ratio: $OLTP_RATIO (target 40-60% CPU)"
echo "QPS: $QPS (0=unlimited)"
echo ""

# Kill any existing server on this port
echo "Cleaning up existing processes..."
pkill -f "sqlrustgo.*$PORT" 2>/dev/null || true
sleep 2

# Prepare data directory
echo "Preparing data directory..."
rm -rf /tmp/sqlrustgo-v311-soak
mkdir -p /tmp/sqlrustgo-v311-soak

# Start server with optimized settings
echo "Starting server..."
$BINARY serve \
    --port $PORT \
    --data-dir /tmp/sqlrustgo-v311-soak \
    --storage parallel \
    --wal-sync batch:10000 \
    --server-threads 16 \
    --max-connections 200 \
    --log-level error \
    > "$RESULTS_DIR/server.log" 2>&1 &

SERVER_PID=$!
echo "Server PID: $SERVER_PID"
sleep 5

# Verify server is ready
echo "Waiting for server to be ready..."
for i in {1..30}; do
    if mysql -h $HOST -P $PORT -u root -e "SELECT 1" 2>/dev/null; then
        echo "Server ready!"
        break
    fi
    sleep 1
done

# Create TPC-H schema
echo "Creating TPC-H schema..."
mysql -h $HOST -P $PORT -u root -e "
CREATE DATABASE IF NOT EXISTS tpch;
USE tpch;

CREATE TABLE IF NOT EXISTS region (
    r_regionkey INTEGER PRIMARY KEY,
    r_name TEXT NOT NULL,
    r_comment TEXT
);

CREATE TABLE IF NOT EXISTS nation (
    n_nationkey INTEGER PRIMARY KEY,
    n_name TEXT NOT NULL,
    n_regionkey INTEGER NOT NULL,
    n_comment TEXT
);

CREATE TABLE IF NOT EXISTS supplier (
    s_suppkey INTEGER PRIMARY KEY,
    s_name TEXT NOT NULL,
    s_address TEXT NOT NULL,
    s_nationkey INTEGER NOT NULL,
    s_phone TEXT NOT NULL,
    s_acctbal REAL NOT NULL,
    s_comment TEXT
);

CREATE TABLE IF NOT EXISTS customer (
    c_custkey INTEGER PRIMARY KEY,
    c_name TEXT NOT NULL,
    c_address TEXT NOT NULL,
    c_nationkey INTEGER NOT NULL,
    c_phone TEXT NOT NULL,
    c_acctbal REAL NOT NULL,
    c_mktsegment TEXT NOT NULL,
    c_comment TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS orders (
    o_orderkey INTEGER PRIMARY KEY,
    o_custkey INTEGER NOT NULL,
    o_orderstatus TEXT NOT NULL,
    o_totalprice REAL NOT NULL,
    o_orderdate TEXT NOT NULL,
    o_orderpriority TEXT NOT NULL,
    o_clerk TEXT NOT NULL,
    o_shippriority INTEGER NOT NULL,
    o_comment TEXT
);

CREATE TABLE IF NOT EXISTS lineitem (
    l_orderkey INTEGER NOT NULL,
    l_partkey INTEGER NOT NULL,
    l_suppkey INTEGER NOT NULL,
    l_linenumber INTEGER NOT NULL,
    l_quantity REAL NOT NULL,
    l_extendedprice REAL NOT NULL,
    l_discount REAL NOT NULL,
    l_tax REAL NOT NULL,
    l_returnflag TEXT NOT NULL,
    l_linestatus TEXT NOT NULL,
    l_shipdate TEXT NOT NULL,
    l_commitdate TEXT NOT NULL,
    l_receiptdate TEXT NOT NULL,
    l_shipinstruct TEXT NOT NULL,
    l_shipmode TEXT NOT NULL,
    l_comment TEXT
);

CREATE TABLE IF NOT EXISTS part (
    p_partkey INTEGER PRIMARY KEY,
    p_name TEXT NOT NULL,
    p_mfgr TEXT NOT NULL,
    p_brand TEXT NOT NULL,
    p_type TEXT NOT NULL,
    p_size INTEGER NOT NULL,
    p_container TEXT NOT NULL,
    p_retailprice REAL NOT NULL,
    p_comment TEXT
);

CREATE TABLE IF NOT EXISTS partsupp (
    ps_partkey INTEGER NOT NULL,
    ps_suppkey INTEGER NOT NULL,
    ps_availqty INTEGER NOT NULL,
    ps_supplycost REAL NOT NULL,
    ps_comment TEXT,
    PRIMARY KEY (ps_partkey, ps_suppkey)
);
" 2>/dev/null || echo "Schema creation completed"

# Record initial row counts
echo "Recording baseline..."
INITIAL_ORDERS=$(mysql -sN -h $HOST -P $PORT -u root -e "USE tpch; SELECT COUNT(*) FROM orders" 2>/dev/null || echo "0")
INITIAL_LINEITEM=$(mysql -sN -h $HOST -P $PORT -u root -e "USE tpch; SELECT COUNT(*) FROM lineitem" 2>/dev/null || echo "0")
echo "Initial orders: $INITIAL_ORDERS"
echo "Initial lineitem: $INITIAL_LINEITEM"

# Run hybrid_soak with long connections
echo ""
echo "Starting hybrid_soak (long connection mode)..."
echo "Each thread maintains a persistent connection throughout the test"
echo ""

# Export env vars for hybrid_soak
export HYBRID_HOST=$HOST
export HYBRID_PORT=$PORT
export HYBRID_THREADS=$THREADS
export HYBRID_DURATION=$DURATION
export HYBRID_OLTP_RATIO=$OLTP_RATIO
export HYBRID_REPORT_INTERVAL=60

$HYBRID_SOAK \
    --host $HOST \
    --port $PORT \
    --threads $THREADS \
    --target-qps $QPS \
    --oltp-ratio $OLTP_RATIO \
    --report-interval 60 \
    --duration $DURATION \
    > "$RESULTS_DIR/soak.log" 2>&1

# Record final row counts
echo ""
echo "Recording final counts..."
FINAL_ORDERS=$(mysql -sN -h $HOST -P $PORT -u root -e "USE tpch; SELECT COUNT(*) FROM orders" 2>/dev/null || echo "0")
FINAL_LINEITEM=$(mysql -sN -h $HOST -P $PORT -u root -e "USE tpch; SELECT COUNT(*) FROM lineitem" 2>/dev/null || echo "0")

echo ""
echo "=== SOAK Test Complete ==="
echo "Results: $RESULTS_DIR"
echo "Initial orders: $INITIAL_ORDERS, Final: $FINAL_ORDERS"
echo "Initial lineitem: $INITIAL_LINEITEM, Final: $FINAL_LINEITEM"
echo "Orders delta: $((FINAL_ORDERS - INITIAL_ORDERS))"
echo "Lineitem delta: $((FINAL_LINEITEM - INITIAL_LINEITEM))"

# Cleanup
kill $SERVER_PID 2>/dev/null || true
