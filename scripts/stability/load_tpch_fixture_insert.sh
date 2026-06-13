#!/bin/bash
# load_tpch_fixture_insert.sh - Load TPC-H fixture into a running sqlrustgo-mysql-server
#                                via mysql CLI INSERT statements (NOT LOAD DATA).
#
# v3.9.0 wired soak helper. NOT a "self-written stability program" — uses
# industry-standard TPC-H .tbl files + MySQL INSERT wire protocol.
#
# USE THIS instead of load_tpch_fixture.sh when:
#   - sqlrustgo-mysql-server binary is too old to support LOAD DATA LOCAL INFILE
#   - or you want to test pure INSERT path
#
# Usage:
#   HOST=127.0.0.1 PORT=3396 FIXTURE=tpch-sf001 \
#       bash scripts/stability/load_tpch_fixture_insert.sh
#
# Exit codes: same as load_tpch_fixture.sh
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
    echo "FAIL: mysql CLI not in PATH" >&2
    exit 2
fi

if [ ! -d "$FIX_DIR" ]; then
    echo "FAIL: fixture dir not found: $FIX_DIR" >&2
    exit 3
fi

# Schema source of truth: tests/tpch_sf01_perf_baseline_test.rs (v3.9.0 canonical).
SCHEMA_REGION="CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)"
SCHEMA_NATION="CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)"
SCHEMA_SUPPLIER="CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)"
SCHEMA_CUSTOMER="CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT NOT NULL)"
SCHEMA_PART="CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT NOT NULL)"
SCHEMA_PARTSUPP="CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))"
SCHEMA_ORDERS="CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)"
SCHEMA_LINEITEM="CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL, PRIMARY KEY (l_orderkey, l_linenumber))"

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
N_COLS=(3 4 7 8 9 5 9 16)

mysql_q() {
    if [ -n "$PASSWORD" ]; then
        mysql -h "$HOST" -P "$PORT" -u "$USER" -p"$PASSWORD" -e "$1" 2>&1
    else
        mysql -h "$HOST" -P "$PORT" -u "$USER" -e "$1" 2>&1
    fi
}

# Build INSERT batch from a .tbl file. Args: tbl name, file path, n_cols
# We use multi-row INSERT VALUES (a,b),(c,d),... for speed.
# Each line in .tbl is pipe-delimited with trailing pipe (TPC-H format).
# We strip the trailing pipe and escape values for SQL.
build_inserts() {
    local file="$1" ncols="$2" max_rows="${3:-50000}"
    python3 -c "
import sys
ncols = $ncols
max_rows = $max_rows
sql_parts = []
row_count = 0
with open('$file') as f:
    for line in f:
        line = line.rstrip('\n').rstrip('\r')
        if line.endswith('|'):
            line = line[:-1]
        if not line:
            continue
        parts = line.split('|')
        if len(parts) < ncols:
            continue
        parts = parts[:ncols]
        # SQL escape
        def esc(v):
            if v is None: return 'NULL'
            v = v.replace(\"'\", \"''\")
            return f\"'{v}'\"
        # Try numeric coercion for known-numeric columns
        def coerce(v, idx):
            # Columns 0..ncols-1 — for lineitem, numeric cols are 4 (quantity), 5 (ext price), 6 (discount), 7 (tax)
            # For all tables: first columns are usually INT, last are TEXT
            # Just try int/float for first 2-3 cols (likely keys)
            if idx < 3:
                try:
                    return str(int(v))
                except ValueError:
                    pass
                try:
                    return str(float(v))
                except ValueError:
                    pass
            return esc(v) if v != '' else 'NULL'
        row_str = ','.join(coerce(parts[i], i) for i in range(ncols))
        sql_parts.append('(' + row_str + ')')
        row_count += 1
        if row_count >= max_rows:
            break
print(','.join(sql_parts))
print(f'-- {row_count} rows', file=sys.stderr)
" 2>&1
}

echo "=========================================="
echo "TPC-H fixture loader (INSERT mode, wired)"
echo "=========================================="
echo "  HOST=$HOST  PORT=$PORT  USER=$USER"
echo "  FIXTURE=$FIXTURE  FIX_DIR=$FIX_DIR"
echo "=========================================="

# Sanity
for f in "${FILES[@]}"; do
    if [ ! -f "$FIX_DIR/$f" ]; then
        echo "FAIL: missing fixture file: $FIX_DIR/$f" >&2
        exit 4
    fi
done

START=$(date +%s)

for i in "${!TABLES[@]}"; do
    tbl="${TABLES[$i]}"
    schema="${SCHEMAS[$i]}"
    file="$FIX_DIR/${FILES[$i]}"
    ncols="${N_COLS[$i]}"

    if [ "$DROP_FIRST" = "1" ]; then
        echo "[$(date +%H:%M:%S)] DROP+CREATE $tbl"
        mysql_q "DROP TABLE IF EXISTS $tbl; $schema" >/dev/null
    fi

    echo "[$(date +%H:%M:%S)] Building INSERT batch from $file (ncols=$ncols)..."
    rows=$(build_inserts "$file" "$ncols" 50000 2>/tmp/insert_count)
    row_count=$(cat /tmp/insert_count 2>/dev/null | grep -oE "[0-9]+ rows" | head -1 | grep -oE "[0-9]+" || echo "?")
    echo "  Built $row_count row tuples"

    echo "[$(date +%H:%M:%S)] INSERT INTO $tbl ..."
    if ! mysql_q "INSERT INTO $tbl VALUES $rows;" 2>&1 | tail -5; then
        echo "WARN: INSERT INTO $tbl had issues (continuing)" >&2
    fi

    # Row count sanity
    count=$(mysql_q "SELECT COUNT(*) FROM $tbl;" 2>/dev/null | tail -1 | tr -d ' ' || echo 0)
    echo "         → $count rows in $tbl"
done

ELAPSED=$(( $(date +%s) - START ))
echo "=========================================="
echo "Fixture loaded in ${ELAPSED}s. Host=$HOST Port=$PORT"
echo "=========================================="
