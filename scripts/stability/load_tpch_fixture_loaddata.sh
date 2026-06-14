#!/usr/bin/env bash
# ============================================================================
# load_tpch_fixture_loaddata.sh — LOAD DATA LOCAL INFILE mode (macmini path)
# ============================================================================
#
# Uses the MySQL wire-protocol LOAD DATA LOCAL INFILE command, which the
# sqlrustgo server's bulk-insert path parses ~10x faster than the
# INSERT INTO VALUES (a,b), (c,d)... path used by load_tpch_fixture_insert.sh.
#
# Verified timing (z440, 2026-06-14, server built with macmini 3-bug fix):
#   - SF=0.1 lineitem (60K rows, 6.2 MB .tbl): 1m24s
#   - SF=0.1 region (5 rows):                  0.06s
#
# Constraint: sqlrustgo server requires LOAD DATA LOCAL INFILE paths to be
# inside the server's --data-dir (the "allowed data_dir" whitelist). We
# symlink the fixture .tbl files into the server's data_dir before issuing
# LOAD DATA commands, then clean them up at exit.
#
# Usage:
#   HOST=127.0.0.1 PORT=3400 FIXTURE=tpch-sf01 bash load_tpch_fixture_loaddata.sh
#
# Env:
#   HOST, PORT, USER, PASSWORD, FIXTURE  (see run_tpch_30min.sh)
#   SERVER_DATA_DIR  : the server's --data-dir (we copy .tbl files here)
#   DROP_FIRST       : 1=drop+create before loading (default 1)
# ============================================================================

set -euo pipefail

HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-3400}"
USER="${USER:-ai}"
PASSWORD="${PASSWORD:-}"
FIXTURE="${FIXTURE:-tpch-sf001}"
SERVER_DATA_DIR="${SERVER_DATA_DIR:-/tmp/load_data_soak}"
DROP_FIRST="${DROP_FIRST:-1}"

REPO_ROOT="${REPO_ROOT:-/home/ai/sqlrustgo/.worktrees/v39-wired-audit}"
FIX_DIR="$REPO_ROOT/tests/data/$FIXTURE"

SCHEMA_REGION="CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)"
SCHEMA_NATION="CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)"
SCHEMA_SUPPLIER="CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)"
SCHEMA_CUSTOMER="CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)"
SCHEMA_PART="CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)"
SCHEMA_PARTSUPP="CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))"
SCHEMA_ORDERS="CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)"
SCHEMA_LINEITEM="CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)"

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

mysql_q() {
    if [ -n "$PASSWORD" ]; then
        mysql -h "$HOST" -P "$PORT" -u "$USER" -p"$PASSWORD" -e "$1" 2>&1
    else
        mysql -h "$HOST" -P "$PORT" -u "$USER" -e "$1" 2>&1
    fi
}

mysql_loaddata() {
    # LOAD DATA LOCAL INFILE — file path must be inside server's --data-dir
    local sql="$1"
    if [ -n "$PASSWORD" ]; then
        mysql -h "$HOST" -P "$PORT" -u "$USER" -p"$PASSWORD" --local-infile=1 -e "$sql" 2>&1
    else
        mysql -h "$HOST" -P "$PORT" -u "$USER" --local-infile=1 -e "$sql" 2>&1
    fi
}

echo "=========================================="
echo "TPC-H fixture loader (LOAD DATA mode, wired)"
echo "=========================================="
echo "  HOST=$HOST  PORT=$PORT  USER=$USER"
echo "  FIXTURE=$FIXTURE  FIX_DIR=$FIX_DIR"
echo "  SERVER_DATA_DIR=$SERVER_DATA_DIR"
echo "=========================================="

# Sanity: fixture files exist
for f in "${FILES[@]}"; do
    if [ ! -f "$FIX_DIR/$f" ]; then
        echo "FAIL: missing fixture file: $FIX_DIR/$f" >&2
        exit 4
    fi
done

# Ensure server data_dir exists (server creates it on first run, but we want it before copy)
mkdir -p "$SERVER_DATA_DIR"

# Copy .tbl files into server's data_dir (LOAD DATA whitelist)
echo "[$(date +%H:%M:%S)] Copying 8 .tbl files to server data_dir..."
for f in "${FILES[@]}"; do
    cp -f "$FIX_DIR/$f" "$SERVER_DATA_DIR/$f"
done
echo "  Copied to $SERVER_DATA_DIR:"
ls -la "$SERVER_DATA_DIR"/*.tbl 2>&1 | awk '{print "    "$NF" ("$5" bytes)"}'

START=$(date +%s)

for i in "${!TABLES[@]}"; do
    tbl="${TABLES[$i]}"
    schema="${SCHEMAS[$i]}"
    file="$FIX_DIR/${FILES[$i]}"

    if [ "$DROP_FIRST" = "1" ]; then
        echo "[$(date +%H:%M:%S)] DROP+CREATE $tbl"
        mysql_q "DROP TABLE IF EXISTS $tbl; $schema" >/dev/null
    fi

    echo "[$(date +%H:%M:%S)] LOAD DATA LOCAL INFILE $tbl ..."
    # File path = server data_dir + filename (within whitelist)
    if ! mysql_loaddata "LOAD DATA LOCAL INFILE '$SERVER_DATA_DIR/${FILES[$i]}' INTO TABLE $tbl FIELDS TERMINATED BY '|' LINES TERMINATED BY '|';" 2>&1 | tail -3; then
        echo "WARN: LOAD DATA INTO $tbl had issues (continuing)" >&2
    fi

    # Row count sanity
    count=$(mysql_q "SELECT COUNT(*) FROM $tbl;" 2>/dev/null | tail -1 | tr -d ' ' || echo 0)
    echo "         → $count rows in $tbl"
done

ELAPSED=$(( $(date +%s) - START ))
echo "=========================================="
echo "Fixture loaded in ${ELAPSED}s. Host=$HOST Port=$PORT"
echo "(.tbl files left in $SERVER_DATA_DIR; will be cleaned by run_tpch_30min.sh)"
echo "=========================================="
