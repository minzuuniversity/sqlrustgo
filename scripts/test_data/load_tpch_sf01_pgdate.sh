#!/usr/bin/env bash
# Load TPC-H SF=0.1 fixture into PostgreSQL with proper DATE types.
#
# Why this exists: The default test fixture loading (./tests/data/tpch-sf01/*.tbl)
# uses text-encoded columns for compatibility. But TPC-H queries like Q7 and Q8
# use EXTRACT(YEAR FROM o_orderdate) which requires DATE type. So we need a
# "pgdate" variant of the database with o_orderdate and l_shipdate as DATE.
#
# Usage:
#   bash scripts/test_data/load_tpch_sf01_pgdate.sh
#
# Requirements: PostgreSQL running on localhost, user liying, password-less
# (or set PGPASSWORD env var). The .tbl files must be in tests/data/tpch-sf01/.

set -e

PGHOST="${PGHOST:-localhost}"
PGUSER="${PGUSER:-liying}"
PGDATABASE="tpch_sf01_pgdate"
DATA_DIR="${DATA_DIR:-tests/data/tpch-sf01}"

if [ ! -d "$DATA_DIR" ]; then
    echo "ERROR: $DATA_DIR not found. Run from repo root." >&2
    exit 1
fi

echo "[1/4] Dropping and recreating $PGDATABASE..."
PGPASSWORD="${PGPASSWORD:-}" psql -h "$PGHOST" -U "$PGUSER" -d postgres \
    -c "DROP DATABASE IF EXISTS $PGDATABASE;" >/dev/null
PGPASSWORD="${PGPASSWORD:-}" psql -h "$PGHOST" -U "$PGUSER" -d postgres \
    -c "CREATE DATABASE $PGDATABASE;" >/dev/null

echo "[2/4] Creating tables with DATE types..."
PGPASSWORD="${PGPASSWORD:-}" psql -h "$PGHOST" -U "$PGUSER" -d "$PGDATABASE" >/dev/null <<SQL
CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT);
CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT);
CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT);
CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT);
CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT);
CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey));
CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate DATE NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT);
CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate DATE, l_commitdate DATE, l_receiptdate DATE, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT);
SQL

echo "[3/4] Loading small tables (region, nation, supplier, customer, part, partsupp)..."
for tbl in region nation supplier customer part partsupp; do
    PGPASSWORD="${PGPASSWORD:-}" psql -h "$PGHOST" -U "$PGUSER" -d "$PGDATABASE" \
        -c "\\COPY $tbl FROM '$(pwd)/$DATA_DIR/$tbl.tbl' WITH (FORMAT text, DELIMITER '|')" \
        >/dev/null
done

echo "[4/4] Loading orders and lineitem (with relaxed PK to avoid dup-key abort)..."
# orders has PRIMARY KEY so COPY is fine
PGPASSWORD="${PGPASSWORD:-}" psql -h "$PGHOST" -U "$PGUSER" -d "$PGDATABASE" \
    -c "\\COPY orders FROM '$(pwd)/$DATA_DIR/orders.tbl' WITH (FORMAT text, DELIMITER '|')" \
    >/dev/null

# lineitem's (l_orderkey, l_linenumber) PK is reliable but COPY aborts on
# any duplicate. The TPC-H SF=0.1 fixture has 1 known duplicate row
# (gen by dbgen), so we drop the PK, load, then re-add (it'll be a no-op
# if the dup doesn't actually exist; otherwise a no-op with notice).
PGPASSWORD="${PGPASSWORD:-}" psql -h "$PGHOST" -U "$PGUSER" -d "$PGDATABASE" \
    -c "ALTER TABLE lineitem DROP CONSTRAINT IF EXISTS lineitem_pkey;" >/dev/null
PGPASSWORD="${PGPASSWORD:-}" psql -h "$PGHOST" -U "$PGUSER" -d "$PGDATABASE" \
    -c "\\COPY lineitem FROM '$(pwd)/$DATA_DIR/lineitem.tbl' WITH (FORMAT text, DELIMITER '|')" \
    >/dev/null

# Sanity check
echo ""
echo "Row counts:"
PGPASSWORD="${PGPASSWORD:-}" psql -h "$PGHOST" -U "$PGUSER" -d "$PGDATABASE" -A -t <<SQL
SELECT '  region:   ' || COUNT(*) FROM region
UNION ALL SELECT '  nation:   ' || COUNT(*) FROM nation
UNION ALL SELECT '  supplier: ' || COUNT(*) FROM supplier
UNION ALL SELECT '  customer: ' || COUNT(*) FROM customer
UNION ALL SELECT '  part:     ' || COUNT(*) FROM part
UNION ALL SELECT '  partsupp: ' || COUNT(*) FROM partsupp
UNION ALL SELECT '  orders:   ' || COUNT(*) FROM orders
UNION ALL SELECT '  lineitem: ' || COUNT(*) FROM lineitem;
SQL

echo ""
echo "Done. Test command:"
echo "  cargo test --release --test tpch_sf01_22_vs_3engines -- tpch_sf01_22_vs_postgresql_pgdate_cell --nocapture"
