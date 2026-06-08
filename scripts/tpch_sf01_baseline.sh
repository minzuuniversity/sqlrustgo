#!/bin/bash
# tpch_sf01_baseline.sh - Generate 22-query baseline from SQLite on SF=0.1 fixture
#
# Loads tests/data/tpch-sf01/*.tbl into a temp SQLite DB (with the
# TPC-H SF=0.1 schema), runs all 22 TPC-H queries, and emits
# tests/data/tpch-sf01/expected/Q{N}_sf01_baseline.json with
# {row_count, first_3_rows} for each query.
#
# Usage:
#   bash scripts/tpch_sf01_baseline.sh
#
# Companion to tests/tpch_sf01_inprocess_test.rs (which uses sqlrustgo).
# Together they provide a 3-way cross-engine check: sqlrustgo in-process
# vs SQLite (this script) vs wire test on SF=0.1.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DATA_DIR="$PROJECT_ROOT/tests/data/tpch-sf01"
QUERIES_DIR="$PROJECT_ROOT/queries"
EXPECTED_DIR="$DATA_DIR/expected"

if [ ! -d "$DATA_DIR" ]; then
    echo "[ERROR] $DATA_DIR not found. Run scripts/setup_tpch_sf01.sh first." >&2
    exit 1
fi

mkdir -p "$EXPECTED_DIR"

# Build CREATE TABLE statements (matches tests/queries/q*.sql DDL)
cat > /tmp/tpch_sf01_schema.sql << 'SQL'
CREATE TABLE region  (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT);
CREATE TABLE nation  (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT);
CREATE TABLE supplier(s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT);
CREATE TABLE customer(c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT);
CREATE TABLE part    (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT);
CREATE TABLE partsupp(ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT);
CREATE TABLE orders  (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT);
CREATE TABLE lineitem(l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT);
SQL

# Load data into temp SQLite
DB=$(mktemp -t tpch_sf01_XXXXXX).db
trap "rm -f $DB /tmp/tpch_sf01_schema.sql" EXIT
echo "=== Loading SF=0.1 into $DB ==="
sqlite3 "$DB" < /tmp/tpch_sf01_schema.sql
python3 -c "
import sqlite3, sys
db = sys.argv[1]
data = sys.argv[2]
con = sqlite3.connect(db)
cur = con.cursor()
for tbl, sql_types in [
    ('region',   'INTEGER, TEXT, TEXT'),
    ('nation',   'INTEGER, TEXT, INTEGER, TEXT'),
    ('supplier', 'INTEGER, TEXT, TEXT, INTEGER, TEXT, REAL, TEXT'),
    ('customer', 'INTEGER, TEXT, TEXT, INTEGER, TEXT, REAL, TEXT, TEXT'),
    ('part',     'INTEGER, TEXT, TEXT, TEXT, TEXT, INTEGER, TEXT, REAL, TEXT'),
    ('partsupp', 'INTEGER, INTEGER, INTEGER, REAL, TEXT'),
    ('orders',   'INTEGER, INTEGER, TEXT, REAL, TEXT, TEXT, TEXT, INTEGER, TEXT'),
    ('lineitem', 'INTEGER, INTEGER, INTEGER, INTEGER, REAL, REAL, REAL, REAL, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT'),
]:
    path = f'{data}/{tbl}.tbl'
    inserted = 0
    with open(path) as f:
        for line in f:
            line = line.rstrip('\n')
            if line.endswith('|'):
                line = line[:-1]
            fields = line.split('|')
            if tbl == 'region':
                vals = (int(fields[0]), fields[1], fields[2])
            elif tbl == 'nation':
                vals = (int(fields[0]), fields[1], int(fields[2]), fields[3])
            elif tbl == 'supplier':
                vals = (int(fields[0]), fields[1], fields[2], int(fields[3]), fields[4], float(fields[5]), fields[6])
            elif tbl == 'customer':
                vals = (int(fields[0]), fields[1], fields[2], int(fields[3]), fields[4], float(fields[5]), fields[6], fields[7])
            elif tbl == 'part':
                vals = (int(fields[0]), fields[1], fields[2], fields[3], fields[4], int(fields[5]), fields[6], float(fields[7]), fields[8])
            elif tbl == 'partsupp':
                vals = (int(fields[0]), int(fields[1]), int(fields[2]), float(fields[3]), fields[4])
            elif tbl == 'orders':
                vals = (int(fields[0]), int(fields[1]), fields[2], float(fields[3]), fields[4], fields[5], fields[6], int(fields[7]), fields[8])
            elif tbl == 'lineitem':
                vals = (int(fields[0]), int(fields[1]), int(fields[2]), int(fields[3]), float(fields[4]), float(fields[5]), float(fields[6]), float(fields[7]), fields[8], fields[9], fields[10], fields[11], fields[12], fields[13], fields[14], fields[15])
            else:
                continue
            placeholders = ','.join('?' * len(vals))
            cur.execute(f'INSERT INTO {tbl} VALUES ({placeholders})', vals)
            inserted += 1
    con.commit()
    print(f'  {tbl}: {inserted} rows')
con.close()
" "$DB" "$DATA_DIR"

# Run all 22 queries
echo "=== Running 22 TPC-H queries ==="
PASS=0
FAIL=0
for n in $(seq 1 22); do
    sql=$(cat "$QUERIES_DIR/q${n}.sql")
    sql_stripped="${sql%;}"  # strip trailing semicolon for wrapping
    # Run the query to a temp file (preserves trailing newlines that
    # shell command substitution would strip — important to
    # distinguish 0 rows from 1 row with NULL).
    out_file=$(mktemp -t tpch_baseline_XXXXXX)
    sqlite3 -separator "|" "$DB" "$sql" > "$out_file" 2>&1 || {
        echo "  Q${n}: ERROR (sqlite failed)"
        FAIL=$((FAIL + 1))
        rm -f "$out_file"
        continue
    }
    # Use a wrapper COUNT to get the true row count (handles empty
    # result vs 1-NULL-row case correctly).
    rc=$(sqlite3 "$DB" "SELECT count(*) FROM ($sql_stripped)" 2>/dev/null)
    if [ -z "$rc" ]; then rc=0; fi
    out=$(cat "$out_file")
    rm -f "$out_file"
    if [ "$rc" -eq 0 ]; then
        # Empty result (no rows) — sqlite output is blank
        rc=0
        first3='[]'
    else
        first3=$(echo "$out" | head -3 | python3 -c "
import json, sys
rows = [line.strip().split('|') for line in sys.stdin if line.strip()]
print(json.dumps(rows))
")
    fi
    # Write expected JSON
    python3 -c "
import json
with open('$EXPECTED_DIR/Q${n}_sf01_baseline.json', 'w') as f:
    json.dump({
        'query': $n,
        'row_count': $rc,
        'first_3_rows': $first3,
    }, f, indent=2)
"
    echo "  Q${n}: rc=$rc"
    PASS=$((PASS + 1))
done
echo
echo "=== Baseline: $PASS queries captured ==="
echo "Output: $EXPECTED_DIR/"
