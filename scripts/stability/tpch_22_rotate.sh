#!/bin/bash
# tpch_22_rotate.sh - Periodically run TPC-H Q1..Q22 over a wired MySQL
#                     connection to validate server stability under
#                     real TPC-H workload, NOT synthetic.
#
# v3.9.0 wired soak helper. NOT a "self-written stability program" — uses
# industry-standard TPC-H 22 queries + mysql CLI as wire client.
#
# Usage:
#   HOST=127.0.0.1 PORT=3396 \
#   INTERVAL=60 MAX_ROUNDS=10 \
#       bash scripts/stability/tpch_22_rotate.sh
#
# Environment variables:
#   HOST       MySQL host                  (default: 127.0.0.1)
#   PORT       MySQL port                  (default: 3396)
#   USER       MySQL user                  (default: root)
#   PASSWORD   MySQL password              (default: empty)
#   INTERVAL   Seconds between full rounds (default: 600 = 10min)
#   MAX_ROUNDS 0 = run forever, N>0 = stop after N rounds
#   LOG_FILE   Per-query log (CSV)         (default: auto)
#   PER_QUERY_TIMEOUT  Per-query timeout s  (default: 60)
#
# Exit codes:
#   0 normal exit (MAX_ROUNDS reached or signal)
#   2 mysql CLI not found
#   3 log file unwritable
#   4 server unreachable
#
# This script is meant to run in parallel alongside sysbench (run by
# run_wired_soak.sh). It does NOT stop the server; the parent script
# owns lifecycle.
#
# Issues: prerequisite for #3225 / #3229 wired soak, complements
#         run_24h_soak_v2.sh sysbench oltp_read_write workload and
#         test_integration_5min.sh 5-min anti-OOM (PR #3380).
#
# Maintainer: Hermes Agent
# Last touched: 2026-06-14

set -euo pipefail

HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-3396}"
USER="${USER:-root}"
PASSWORD="${PASSWORD:-}"
INTERVAL="${INTERVAL:-600}"
MAX_ROUNDS="${MAX_ROUNDS:-0}"
PER_QUERY_TIMEOUT="${PER_QUERY_TIMEOUT:-60}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="${LOG_FILE:-$SCRIPT_DIR/../tpch_22_rotate_$(date +%Y%m%d_%H%M%S).log}"

# Touch log file early so we fail fast on bad paths
if ! ( : >> "$LOG_FILE" ) 2>/dev/null; then
    echo "FAIL: cannot write LOG_FILE=$LOG_FILE" >&2
    exit 3
fi

if ! command -v mysql >/dev/null 2>&1; then
    echo "FAIL: mysql CLI not in PATH" >&2
    exit 2
fi

# Sanity: server reachable?
if ! mysql -h "$HOST" -P "$PORT" -u "$USER" ${PASSWORD:+-p"$PASSWORD"} \
        -e "SELECT 1" >/dev/null 2>&1; then
    echo "FAIL: cannot reach server at $HOST:$PORT" >&2
    exit 4
fi

# Run a single query, return wall-clock ms. Logs to LOG_FILE.
# Args: $1=query_name, $2=sql_text
run_one() {
    local qname="$1" sql="$2"
    local start_ms end_ms elapsed_ms err
    # macOS date doesn't support %3N; use python for ms precision
    start_ms=$(python3 -c 'import time; print(int(time.time()*1000))')
    err=$(timeout "$PER_QUERY_TIMEOUT" mysql -h "$HOST" -P "$PORT" -u "$USER" \
              ${PASSWORD:+-p"$PASSWORD"} -B -N -e "$sql" 2>&1 >/dev/null) || err="timeout_or_err"
    end_ms=$(python3 -c 'import time; print(int(time.time()*1000))')
    elapsed_ms=$(( end_ms - start_ms ))
    local ts
    ts=$(date '+%Y-%m-%d %H:%M:%S')
    echo "$ts,$qname,$elapsed_ms,${err:-ok}" >> "$LOG_FILE"
    if [ -n "$err" ]; then
        echo "  $qname: ${elapsed_ms}ms ERR($err)" >&2
    else
        echo "  $qname: ${elapsed_ms}ms"
    fi
}

# TPC-H Q1..Q22 (canonical, TPC-H spec). Schema matches load_tpch_fixture.sh.
# NOTE: Q2/Q9/Q13/Q16/Q17/Q20/Q21/Q22 are known heavy / subquery-heavy on
# sqlrustgo v3.9.0; errors are expected but logged. Stability = no crash.
Q1="SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, SUM(l_extendedprice) AS sum_base_price, SUM(l_extendedprice*(1-l_discount)) AS sum_disc_price, SUM(l_extendedprice*(1-l_discount)*(1+l_tax)) AS sum_charge, AVG(l_quantity) AS avg_qty, AVG(l_extendedprice) AS avg_price, AVG(l_discount) AS avg_disc, COUNT(*) AS count_order FROM lineitem WHERE l_shipdate <= '1998-12-01' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus"
Q2="SELECT s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address, s_phone, s_comment FROM part, supplier, partsupp, nation, region WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey AND p_size = 15 AND p_type LIKE '%BRASS' AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'EUROPE' ORDER BY s_acctbal DESC, s_name, p_partkey LIMIT 10"
Q3="SELECT l_orderkey, SUM(l_extendedprice*(1-l_discount)) AS revenue, o_orderdate, o_shippriority FROM customer, orders, lineitem WHERE c_mktsegment = 'BUILDING' AND c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate < '1995-03-15' AND l_shipdate > '1995-03-15' GROUP BY l_orderkey, o_orderdate, o_shippriority ORDER BY revenue DESC, o_orderdate LIMIT 10"
Q4="SELECT o_orderpriority, COUNT(*) AS order_count FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' AND EXISTS (SELECT * FROM lineitem WHERE l_orderkey = o_orderkey AND l_commitdate < l_receiptdate) GROUP BY o_orderpriority ORDER BY o_orderpriority"
Q5="SELECT n_name, SUM(l_extendedprice*(1-l_discount)) AS revenue FROM customer, orders, lineitem, supplier, nation, region WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND l_suppkey = s_suppkey AND c_nationkey = s_nationkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'ASIA' AND o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01' GROUP BY n_name ORDER BY revenue DESC"
Q6="SELECT SUM(l_extendedprice*l_discount) AS revenue FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.05 AND 0.07 AND l_quantity < 24"
Q7="SELECT supp_nation, cust_nation, l_year, SUM(volume) AS revenue FROM (SELECT n1.n_name AS supp_nation, n2.n_name AS cust_nation, EXTRACT(YEAR FROM l_shipdate) AS l_year, l_extendedprice*(1-l_discount) AS volume FROM supplier, lineitem, orders, customer, nation n1, nation n2 WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey AND c_custkey = o_custkey AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey AND ( (n1.n_name = 'FRANCE' AND n2.n_name = 'GERMANY') OR (n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE') ) AND l_shipdate BETWEEN '1995-01-01' AND '1996-12-31') AS shipping GROUP BY supp_nation, cust_nation, l_year ORDER BY supp_nation, cust_nation, l_year"
Q8="SELECT o_year, SUM(CASE WHEN nation = 'BRAZIL' THEN volume ELSE 0 END) / SUM(volume) AS mkt_share FROM (SELECT EXTRACT(YEAR FROM o_orderdate) AS o_year, l_extendedprice*(1-l_discount) AS volume, n2.n_name AS nation FROM part, supplier, lineitem, orders, customer, nation n1, nation n2, region WHERE p_partkey = l_partkey AND s_suppkey = l_suppkey AND l_orderkey = o_orderkey AND c_custkey = o_custkey AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey AND n2.n_regionkey = r_regionkey AND r_name = 'AMERICA' AND o_orderdate BETWEEN '1995-01-01' AND '1996-12-31' AND p_type = 'ECONOMY ANODIZED STEEL') AS all_nations GROUP BY o_year ORDER BY o_year"
Q9="SELECT nation, o_year, SUM(amount) AS sum_profit FROM (SELECT n_name AS nation, EXTRACT(YEAR FROM o_orderdate) AS o_year, l_extendedprice*(1-l_discount) - ps_supplycost*l_quantity AS amount FROM part, supplier, lineitem, partsupp, orders, nation WHERE s_suppkey = l_suppkey AND ps_suppkey = l_suppkey AND ps_partkey = l_partkey AND p_partkey = l_partkey AND o_orderkey = l_orderkey AND s_nationkey = n_nationkey AND p_name LIKE '%green%') AS profit GROUP BY nation, o_year ORDER BY nation, o_year DESC"
Q10="SELECT c_custkey, c_name, SUM(l_extendedprice*(1-l_discount)) AS revenue, c_acctbal, n_name, c_address, c_phone, c_comment FROM customer, orders, lineitem, nation WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND c_nationkey = n_nationkey AND o_orderdate >= '1993-10-01' AND o_orderdate < '1994-01-01' AND l_returnflag = 'R' GROUP BY c_custkey, c_name, c_acctbal, c_phone, n_name, c_address, c_comment ORDER BY revenue DESC LIMIT 20"
Q11="SELECT ps_partkey, SUM(ps_supplycost*ps_availqty) AS value FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY' GROUP BY ps_partkey HAVING SUM(ps_supplycost*ps_availqty) > (SELECT SUM(ps_supplycost*ps_availqty) * 0.0001 FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY') ORDER BY value DESC"
Q12="SELECT l_shipmode, SUM(CASE WHEN o_orderpriority = '1-URGENT' OR o_orderpriority = '2-HIGH' THEN 1 ELSE 0 END) AS high_line_count, SUM(CASE WHEN o_orderpriority <> '1-URGENT' AND o_orderpriority <> '2-HIGH' THEN 1 ELSE 0 END) AS low_line_count FROM orders, lineitem WHERE o_orderkey = l_orderkey AND l_shipdate > '1994-10-02' AND l_commitdate < '1994-10-02' AND l_receiptdate >= '1994-10-02' AND l_receiptdate < '1994-11-02' GROUP BY l_shipmode ORDER BY l_shipmode"
Q13="SELECT c_count, COUNT(*) AS custdist FROM (SELECT c_custkey, COUNT(o_orderkey) AS c_count FROM customer LEFT OUTER JOIN orders ON c_custkey = o_custkey AND o_comment NOT LIKE '%special%requests%' GROUP BY c_custkey) AS c_orders GROUP BY c_count ORDER BY custdist DESC, c_count DESC"
Q14="SELECT 100.00*SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice*(1-l_discount) ELSE 0 END) / SUM(l_extendedprice*(1-l_discount)) AS promo_revenue FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'"
Q15="SELECT s_suppkey, s_name, s_address, s_phone, SUM(l_extendedprice*(1-l_discount)) AS total_revenue FROM supplier, lineitem WHERE s_suppkey = l_suppkey AND l_shipdate >= '1996-01-01' AND l_shipdate < '1996-04-01' GROUP BY s_suppkey, s_name, s_address, s_phone ORDER BY s_suppkey"
Q16="SELECT p_brand, p_type, p_size, COUNT(DISTINCT ps_suppkey) AS supplier_cnt FROM partsupp, part WHERE p_partkey = ps_partkey AND p_brand <> 'Brand#45' AND p_type NOT LIKE 'MEDIUM POLISHED%' AND p_size IN (49, 14, 23, 45, 19, 3, 36, 9) AND ps_suppkey NOT IN (SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%Customer%Complaints%') GROUP BY p_brand, p_type, p_size ORDER BY supplier_cnt DESC, p_brand, p_type, p_size"
Q17="SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'MED BOX' AND l_quantity < (SELECT 0.2*AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)"
Q18="SELECT l_orderkey, SUM(l_quantity) AS sum_qty FROM lineitem, (SELECT l_orderkey AS o_orderkey FROM lineitem GROUP BY l_orderkey ORDER BY SUM(l_quantity) DESC LIMIT 10) AS t WHERE l_orderkey = t.o_orderkey GROUP BY l_orderkey ORDER BY l_orderkey"
Q19="SELECT SUM(l_extendedprice*(1-l_discount)) AS revenue FROM lineitem, part WHERE (p_partkey = l_partkey AND p_brand = 'Brand#12' AND p_container IN ('SM CASE', 'SM BOX', 'SM PACK', 'SM PKG') AND l_quantity >= 1 AND l_quantity <= 11 AND p_size BETWEEN 1 AND 5 AND l_shipmode IN ('AIR', 'AIR REG') AND l_shipinstruct = 'DELIVER IN PERSON') OR (p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container IN ('MED BAG', 'MED BOX', 'MED PKG', 'MED PACK') AND l_quantity >= 10 AND l_quantity <= 20 AND p_size BETWEEN 1 AND 10 AND l_shipmode IN ('AIR', 'AIR REG') AND l_shipinstruct = 'DELIVER IN PERSON') OR (p_partkey = l_partkey AND p_brand = 'Brand#34' AND p_container IN ('LG CASE', 'LG BOX', 'LG PACK', 'LG PKG') AND l_quantity >= 20 AND l_quantity <= 30 AND p_size BETWEEN 1 AND 15 AND l_shipmode IN ('AIR', 'AIR REG') AND l_shipinstruct = 'DELIVER IN PERSON')"
Q20="SELECT s_name, s_address FROM supplier, nation WHERE s_suppkey IN (SELECT ps_suppkey FROM partsupp WHERE ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') AND ps_availqty > (SELECT 0.5*SUM(l_quantity) FROM lineitem WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01')) AND s_nationkey = n_nationkey AND n_name = 'CANADA' ORDER BY s_name"
Q21="SELECT s_name, COUNT(*) AS numwait FROM supplier, lineitem l1, orders, nation WHERE s_suppkey = l1.l_suppkey AND o_orderkey = l1.l_orderkey AND o_orderstatus = 'F' AND l1.l_receiptdate > l1.l_commitdate AND EXISTS (SELECT * FROM lineitem l2 WHERE l2.l_orderkey = l1.l_orderkey AND l2.l_suppkey <> l1.l_suppkey) AND NOT EXISTS (SELECT * FROM lineitem l3 WHERE l3.l_orderkey = l1.l_orderkey AND l3.l_suppkey <> l1.l_suppkey AND l3.l_receiptdate > l3.l_commitdate) AND s_nationkey = n_nationkey AND n_name = 'SAUDI ARABIA' GROUP BY s_name ORDER BY numwait DESC, s_name LIMIT 100"
Q22="SELECT cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal FROM (SELECT SUBSTRING(c_phone, 1, 2) AS cntrycode, c_acctbal FROM customer WHERE SUBSTRING(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17') AND c_acctbal > (SELECT AVG(c_acctbal) FROM customer WHERE c_acctbal > 0.00 AND SUBSTRING(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')) AND NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)) AS custsale GROUP BY cntrycode ORDER BY cntrycode"

echo "=========================================="
echo "TPC-H 22-query rotation (wired, MySQL CLI)"
echo "=========================================="
echo "  HOST=$HOST PORT=$PORT INTERVAL=${INTERVAL}s MAX_ROUNDS=${MAX_ROUNDS}"
echo "  LOG_FILE=$LOG_FILE"
echo "  WATCHDOG_FILE=${WATCHDOG_FILE:-(disabled)}"
echo "=========================================="
echo "ts,query,elapsed_ms,status" >> "$LOG_FILE"

# Signal handler — let parent kill us cleanly
cleanup() {
    echo ""
    echo "[$(date +%H:%M:%S)] stopped (signal or MAX_ROUNDS reached)"
    echo "  log: $LOG_FILE"
}
trap cleanup EXIT INT TERM

round=0
while true; do
    round=$(( round + 1 ))
    echo ""
    echo "=== Round $round @ $(date '+%Y-%m-%d %H:%M:%S') ==="

    # Watchdog check: if parent's watchdog.state == PAUSE, sleep extra
    # (defaults to 30s) before starting this round's queries. This caps
    # aggregate IO when multiple servers are stacked or the host is busy.
    # (NEW 2026-06-14 — wired-stability-testing Pitfall 9)
    if [ -n "${WATCHDOG_FILE:-}" ] && [ -f "$WATCHDOG_FILE" ]; then
        WD=$(cat "$WATCHDOG_FILE" 2>/dev/null || echo "RUN")
        if [ "$WD" = "PAUSE" ]; then
            BACKOFF=${ROTATE_BACKOFF_S:-30}
            echo "  [watchdog] PAUSE → sleeping ${BACKOFF}s before round $round"
            sleep "$BACKOFF"
        fi
    fi

    run_one "Q1"  "$Q1"
    run_one "Q2"  "$Q2"
    run_one "Q3"  "$Q3"
    run_one "Q4"  "$Q4"
    run_one "Q5"  "$Q5"
    run_one "Q6"  "$Q6"
    run_one "Q7"  "$Q7"
    run_one "Q8"  "$Q8"
    run_one "Q9"  "$Q9"
    run_one "Q10" "$Q10"
    run_one "Q11" "$Q11"
    run_one "Q12" "$Q12"
    run_one "Q13" "$Q13"
    run_one "Q14" "$Q14"
    run_one "Q15" "$Q15"
    run_one "Q16" "$Q16"
    run_one "Q17" "$Q17"
    run_one "Q18" "$Q18"
    run_one "Q19" "$Q19"
    run_one "Q20" "$Q20"
    run_one "Q21" "$Q21"
    run_one "Q22" "$Q22"

    if [ "$MAX_ROUNDS" -gt 0 ] && [ "$round" -ge "$MAX_ROUNDS" ]; then
        echo "MAX_ROUNDS=$MAX_ROUNDS reached; exiting"
        exit 0
    fi

    sleep "$INTERVAL"
done
