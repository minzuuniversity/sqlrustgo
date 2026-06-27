#!/bin/bash
# load_tpch_fixture.sh - Load TPC-H fixture into a running sqlrustgo-mysql-server
#                        via mysql CLI (LOAD DATA LOCAL INFILE).
#
# v3.9.0 wired soak helper. NOT a "self-written stability program" — uses
# industry-standard TPC-H .tbl files + MySQL LOAD DATA wire protocol.
#
# Usage:
#   HOST=127.0.0.1 PORT=3396 FIXTURE=tpch-sf001 \
#       bash scripts/stability/load_tpch_fixture.sh
#
# Environment variables:
#   HOST       MySQL host                (default: 127.0.0.1)
#   PORT       MySQL port                (default: 3396)
#   USER       MySQL user                (default: root)
#   PASSWORD   MySQL password            (default: empty)
#   FIXTURE    tpch-tiny | tpch-sf001    (default: tpch-sf001)
#   FIXTURE_DIR Override fixture dir     (default: <repo>/tests/data/<FIXTURE>)
#   DROP_FIRST 1 = DROP TABLE IF EXISTS  (default: 1)
#
# Exit codes:
#   0 success
#   2 mysql CLI not found
#   3 fixture dir not found
#   4 .tbl file missing
#   5 LOAD DATA failure (per-table)
#
# Issues: prerequisite for #3225 / #3229 wired soak, complements
#         run_24h_soak_v2.sh (sysbench oltp_read_write) and the
#         5-min anti-OOM test (test_integration_5min.sh, PR #3380).
#
# Maintainer: Hermes Agent
# Last touched: 2026-06-14

set -euo pipefail

HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-3396}"
USER="${USER:-root}"
PASSWORD="${PASSWORD:-}"
FIXTURE="${FIXTURE:-tpch-sf001}"
DROP_FIRST="${DROP_FIRST:-1}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [ -n "${FIXTURE_DIR:-}" ]; then
    FIX_DIR="$FIXTURE_DIR"
else
    FIX_DIR="$PROJECT_ROOT/tests/data/$FIXTURE"
fi

if ! command -v mysql >/dev/null 2>&1; then
    echo "FAIL: mysql CLI not in PATH. Install mariadb-client / mysql-client." >&2
    exit 2
fi

if [ ! -d "$FIX_DIR" ]; then
    echo "FAIL: fixture dir not found: $FIX_DIR" >&2
    echo "      Available:" >&2
    ls -1 "$PROJECT_ROOT/tests/data/" 2>/dev/null | sed 's/^/        /' >&2
    exit 3
fi

# Schema source of truth: tests/tpch_sf01_perf_baseline_test.rs (v3.9.0 canonical).
# 8 tables, exact column order matching .tbl files (TPC-H standard | delimited, trailing |).
SCHEMA_REGION="CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)"
SCHEMA_NATION="CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)"
SCHEMA_SUPPLIER="CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)"
SCHEMA_CUSTOMER="CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT NOT NULL)"
SCHEMA_PART="CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT NOT NULL)"
SCHEMA_PARTSUPP="CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))"
SCHEMA_ORDERS="CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)"
SCHEMA_LINEITEM="CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL, PRIMARY KEY (l_orderkey, l_linenumber))"

# Order: small-to-large (FK-safe) — region → nation → supplier/customer/part → partsupp → orders → lineitem
TABLES=(region nation supplier customer part partsupp orders lineitem)
SCHEMAS=(
    "$SCHEMA_REGION"
    "$SCHEMA_NATION"
    "$SCHEMA_SUPPLIER"
    "$SCHEMA_CUSTOMER"
    "$SCHEMA_PART"
    "$SCHEMA_PARTSUPP"
    "$SCHEMA_ORDERS"
    "$SCHEMA_LINEITEM"
)
FILES=(region.tbl nation.tbl supplier.tbl customer.tbl part.tbl partsupp.tbl orders.tbl lineitem.tbl)

echo "=========================================="
echo "TPC-H fixture loader (wired, MySQL LOAD DATA)"
echo "=========================================="
echo "  HOST=$HOST  PORT=$PORT  USER=$USER"
echo "  FIXTURE=$FIXTURE  FIX_DIR=$FIX_DIR"
echo "  DROP_FIRST=$DROP_FIRST"
echo "=========================================="

# Sanity: every .tbl present
for f in "${FILES[@]}"; do
    if [ ! -f "$FIX_DIR/$f" ]; then
        echo "FAIL: missing fixture file: $FIX_DIR/$f" >&2
        exit 4
    fi
done

# Helper: run SQL via mysql CLI (one statement, --local-infile=1)
mysql_q() {
    local sql="$1"
    if [ -n "$PASSWORD" ]; then
        mysql -h "$HOST" -P "$PORT" -u "$USER" -p"$PASSWORD" --local-infile=1 \
              -e "$sql" 2>&1
    else
        mysql -h "$HOST" -P "$PORT" -u "$USER" --local-infile=1 \
              -e "$sql" 2>&1
    fi
}

# Helper: run LOAD DATA via mysql CLI (--local-infile must be on client + server)
mysql_load() {
    local table="$1" file="$2"
    local sql="LOAD DATA LOCAL INFILE '$file' INTO TABLE $table FIELDS TERMINATED BY '|' LINES TERMINATED BY '\n';"
    if [ -n "$PASSWORD" ]; then
        mysql -h "$HOST" -P "$PORT" -u "$USER" -p"$PASSWORD" --local-infile=1 \
              -e "$sql" 2>&1
    else
        mysql -h "$HOST" -P "$PORT" -u "$USER" --local-infile=1 \
              -e "$sql" 2>&1
    fi
}

START=$(date +%s)

for i in "${!TABLES[@]}"; do
    tbl="${TABLES[$i]}"
    schema="${SCHEMAS[$i]}"
    file="$FIX_DIR/${FILES[$i]}"

    if [ "$DROP_FIRST" = "1" ]; then
        echo "[$(date +%H:%M:%S)] DROP+CREATE $tbl"
        mysql_q "DROP TABLE IF EXISTS $tbl; $schema" >/dev/null
    else
        echo "[$(date +%H:%M:%S)] CREATE IF NOT EXISTS $tbl"
        mysql_q "CREATE TABLE IF NOT EXISTS $tbl ($schema_body)" >/dev/null
    fi

    echo "[$(date +%H:%M:%S)] LOAD DATA $tbl from $file"
    if ! mysql_load "$tbl" "$file" >/dev/null; then
        echo "FAIL: LOAD DATA $tbl from $file" >&2
        exit 5
    fi

    # Row count sanity
    count=$(mysql_q "SELECT COUNT(*) FROM $tbl;" 2>/dev/null | tail -1 | tr -d ' ' || echo 0)
    echo "         → $count rows"
done

ELAPSED=$(( $(date +%s) - START ))
echo "=========================================="
echo "Fixture loaded in ${ELAPSED}s. Host=$HOST Port=$PORT"
echo "=========================================="
