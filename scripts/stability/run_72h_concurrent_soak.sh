#!/bin/bash
# run_72h_concurrent_soak.sh - 72h wired SOAK with concurrent bash workers
#
# Launches N concurrent bash workers that continuously execute a TPC-H query
# batch over the wire protocol. Designed for 72-hour stability verification.
#
# Usage:
#   nohup bash run_72h_concurrent_soak.sh > launch.log 2>&1 &
#
# Environment variables (with defaults):
#   HOURS             duration in hours (default: 72)
#   CONCURRENCY       number of concurrent workers (default: 8)
#   HOST              MySQL host (default: 127.0.0.1)
#   PORT              MySQL port (default: 13306)
#   USER              MySQL user (default: root)
#   DATA_DIR          server data directory (default: test_results/wired_soak_72h_<ts>)
#   BATCH_QUERIES     queries per batch (default: 10)
#
# Auto-managed:
#   - Server: sqlrustgo-mysql-server is launched if not already running on PORT
#   - Fixture: TPC-H SF=0.001 fixture loaded if not present
#   - Data persistence: WORKER_STATS written every 30 minutes by all workers
#
# Companion scripts (run in parallel):
#   - monitor_soak.sh: per-10min metrics report
#   - run_wired_soak_guardian.sh: crash auto-restart

set -uo pipefail

HOURS=${HOURS:-72}
CONCURRENCY=${CONCURRENCY:-8}
HOST=${HOST:-127.0.0.1}
PORT=${PORT:-13306}
USER=${USER:-root}
DURATION=$((HOURS * 3600))

# Auto-generate output dir if not set
if [ -z "${DATA_DIR:-}" ]; then
    DATA_DIR="test_results/wired_soak_72h_$(date +%Y%m%d_%H%M%S)"
fi

mkdir -p "$DATA_DIR/data"
BIN="$HOME/sqlrustgo-soak/bin/sqlrustgo-mysql-server"
[ ! -x "$BIN" ] && BIN="./target/release/sqlrustgo-mysql-server"

# Start server if not running
if ! lsof -ti :$PORT >/dev/null 2>&1; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting server on port $PORT..."
    nohup "$BIN" serve \
        --host "$HOST" --port "$PORT" \
        --data-dir "$DATA_DIR/data" \
        --log-level info \
        --server-threads 32 \
        > "$DATA_DIR/sqlrustgo.log" 2>&1 &
    SERVER_PID=$!
    echo $SERVER_PID > "$DATA_DIR/sqlrustgo.pid"
    sleep 3
    # Load TPC-H fixture
    HOST=$HOST PORT=$PORT USER=$USER FIXTURE=tpch-sf001 \
        bash scripts/stability/load_tpch_fixture_insert.sh > "$DATA_DIR/fixture_load.log" 2>&1
fi

# Build batch SQL
cat > "$DATA_DIR/batch.sql" << BATCH
USE default;
SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty FROM lineitem WHERE l_shipdate <= '1998-12-01' GROUP BY l_returnflag, l_linestatus;
SELECT SUM(l_extendedprice*l_discount) AS revenue FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.05 AND 0.07 AND l_quantity < 24;
SELECT l_orderkey, SUM(l_extendedprice*(1-l_discount)) AS revenue FROM customer, orders, lineitem WHERE c_mktsegment = 'BUILDING' AND c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate < '1995-03-15' AND l_shipdate > '1995-03-15' GROUP BY l_orderkey ORDER BY revenue DESC LIMIT 10;
SELECT o_orderpriority, COUNT(*) FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' GROUP BY o_orderpriority;
SELECT n_name, SUM(l_extendedprice*(1-l_discount)) AS revenue FROM customer, orders, lineitem, supplier, nation WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND l_suppkey = s_suppkey AND c_nationkey = s_nationkey AND s_nationkey = n_nationkey GROUP BY n_name;
SELECT ps_partkey, SUM(ps_supplycost*ps_availqty) AS value FROM partsupp, supplier WHERE ps_suppkey = s_suppkey GROUP BY ps_partkey ORDER BY value DESC LIMIT 10;
SELECT l_shipmode, COUNT(*) FROM orders, lineitem WHERE o_orderkey = l_orderkey GROUP BY l_shipmode;
SELECT 100.00*SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice*(1-l_discount) ELSE 0 END) / SUM(l_extendedprice*(1-l_discount)) FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01';
SELECT s_suppkey, s_name FROM supplier, lineitem WHERE s_suppkey = l_suppkey AND l_shipdate >= '1996-01-01' AND l_shipdate < '1996-04-01' GROUP BY s_suppkey, s_name LIMIT 5;
SELECT COUNT(*) FROM customer;
BATCH

echo "[$(date '+%Y-%m-%d %H:%M:%S')] === Starting $HOURS-hour SOAK ==="
echo "  Concurrency: $CONCURRENCY workers"
echo "  Batch: $BATCH_QUERIES queries"
echo "  End time: $(date -v+${DURATION}S '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date -d "+$DURATION seconds" '+%Y-%m-%d %H:%M:%S' 2>/dev/null)"
echo ""

# Launch workers
for i in $(seq 1 $CONCURRENCY); do
    (
        CNT=0
        ERR=0
        END=$((SECONDS + DURATION))
        while [ $SECONDS -lt $END ]; do
            if mysql -h "$HOST" -P "$PORT" -u "$USER" < "$DATA_DIR/batch.sql" >/dev/null 2>&1; then
                CNT=$((CNT + 1))
            else
                ERR=$((ERR + 1))
            fi
        done
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Worker $i: $CNT queries, $ERR errors" >> "$DATA_DIR/worker_stats.txt"
    ) &
done

# Optional: also start TPC-H rotate for wired TPC-H coverage
if [ "${SKIP_TPCH_ROTATE:-0}" = "0" ] && [ -x "./scripts/stability/tpch_22_rotate.sh" ]; then
    LOG_FILE="$DATA_DIR/tpch_22_rotate.log"
    nohup bash ./scripts/stability/tpch_22_rotate.sh \
        HOST="$HOST" PORT="$PORT" USER="$USER" \
        INTERVAL="${TPCH_INTERVAL:-600}" \
        LOG_FILE="$LOG_FILE" \
        >> "$DATA_DIR/tpch_rotate.stdout" 2>&1 &
    echo $! > "$DATA_DIR/tpch_rotate.pid"
fi

# Optional: companion monitor
if [ "${SKIP_MONITOR:-0}" = "0" ] && [ -x "./scripts/stability/monitor_soak.sh" ]; then
    nohup bash ./scripts/stability/monitor_soak.sh "${MONITOR_INTERVAL:-10}" \
        > "$DATA_DIR/monitor.log" 2>&1 &
    echo $! > "$DATA_DIR/monitor.pid"
fi

# Optional: companion guardian
if [ "${SKIP_GUARDIAN:-0}" = "0" ] && [ -x "./scripts/stability/run_wired_soak_guardian.sh" ]; then
    nohup bash ./scripts/stability/run_wired_soak_guardian.sh \
        > "$DATA_DIR/guardian.log" 2>&1 &
    echo $! > "$DATA_DIR/guardian.pid"
fi

# Write launch info
cat > "$DATA_DIR/LAUNCH_INFO.md" << EOF
# $HOURS-hour Concurrent Wired SOAK

- Start: $(date '+%Y-%m-%d %H:%M:%S')
- End: $(date -v+${DURATION}S '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date -d "+$DURATION seconds" '+%Y-%m-%d %H:%M:%S' 2>/dev/null)
- Concurrency: $CONCURRENCY workers
- Server PID: $(lsof -ti :$PORT | head -1)
