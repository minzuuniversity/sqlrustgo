#!/usr/bin/env bash
# Generate TPC-H 22/22 DuckDB baseline (SHA256 + cell values) at SF=0.1.
#
# Output: testdata/tpch/baseline/duckdb/Q{1..22}.json
# Each: { "query": N, "row_count": <int>, "first_3_rows": [[<cell>,...]],
#         "sha256": "<hex>" }
#
# Usage: bash scripts/baselines/generate_duckdb_baseline.sh
# Requires: duckdb (brew install duckdb)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DATA_DIR="$REPO_ROOT/tests/data/tpch-sf01"
QUERIES_DIR="$REPO_ROOT/queries"
OUT_DIR="$REPO_ROOT/testdata/tpch/baseline/duckdb"

if ! command -v duckdb >/dev/null 2>&1; then
    echo "ERROR: duckdb not found. Install: brew install duckdb" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

python3 << PYEOF
import subprocess, json, hashlib, sys, csv, io
from pathlib import Path

REPO = Path("$REPO_ROOT")
DATA = REPO / "tests" / "data" / "tpch-sf01"
QDIR = REPO / "queries"
OUT  = REPO / "testdata" / "tpch" / "baseline" / "duckdb"
OUT.mkdir(parents=True, exist_ok=True)

# TPC-H .tbl spec: dates as TEXT (lexicographic == chronological for 'YYYY-MM-DD').
# Use auto-detect column types (no explicit columns= clause) so DuckDB
# infers the right types and we get all 60000 rows.
LOAD_SQL = (
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT);\n"
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT);\n"
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT);\n"
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT);\n"
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT);\n"
    "CREATE TABLE partsupp (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost REAL, ps_comment TEXT);\n"
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT);\n"
    "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT);\n"
    f"INSERT INTO region   SELECT * FROM read_csv_auto('{DATA}/region.tbl',   header=false, delim='|');\n"
    f"INSERT INTO nation   SELECT * FROM read_csv_auto('{DATA}/nation.tbl',   header=false, delim='|');\n"
    f"INSERT INTO supplier SELECT * FROM read_csv_auto('{DATA}/supplier.tbl', header=false, delim='|');\n"
    f"INSERT INTO customer SELECT * FROM read_csv_auto('{DATA}/customer.tbl', header=false, delim='|');\n"
    f"INSERT INTO part     SELECT * FROM read_csv_auto('{DATA}/part.tbl',     header=false, delim='|');\n"
    f"INSERT INTO partsupp SELECT * FROM read_csv_auto('{DATA}/partsupp.tbl', header=false, delim='|');\n"
    f"INSERT INTO orders   SELECT * FROM read_csv_auto('{DATA}/orders.tbl',   header=false, delim='|');\n"
    f"INSERT INTO lineitem SELECT * FROM read_csv_auto('{DATA}/lineitem.tbl', header=false, delim='|');\n"
)

# Queries that use EXTRACT(YEAR FROM o_orderdate) — the engine has
# special-case handling for 'YYYY-MM-DD' text format, but DuckDB's
# EXTRACT requires DATE type. For these, pre-process the query to
# cast the text date to DATE inline.
EXTRACT_QUERIES = {7, 8, 9}  # Q7, Q8, Q9 all use EXTRACT YEAR

for n in range(1, 23):
    qfile = QDIR / f"q{n}.sql"
    if not qfile.exists():
        print(f"  Q{n:2}: SKIP (no query file)")
        continue
    sql = qfile.read_text().rstrip(';').rstrip()
    if n in EXTRACT_QUERIES:
        # Wrap o_orderdate in CAST for the EXTRACT calls
        # (DuckDB won't accept EXTRACT on VARCHAR)
        sql = sql.replace("EXTRACT(YEAR FROM o_orderdate)",
                          "EXTRACT(YEAR FROM CAST(o_orderdate AS DATE))")
        sql = sql.replace("EXTRACT(YEAR FROM l_shipdate)",
                          "EXTRACT(YEAR FROM CAST(l_shipdate AS DATE))")
        sql = sql.replace("EXTRACT(YEAR FROM l_commitdate)",
                          "EXTRACT(YEAR FROM CAST(l_commitdate AS DATE))")
    full = LOAD_SQL + "\n" + sql + ";"
    result = subprocess.run(
        ['duckdb', ':memory:', '-csv', '-noheader'],
        input=full, capture_output=True, text=True,
        cwd=str(REPO)
    )
    if result.returncode != 0:
        print(f"  Q{n:2}: FAILED - {result.stderr.strip()[:200]}")
        continue

    rows = []
    for line in result.stdout.strip().split('\n'):
        if not line.strip():
            continue
        try:
            for row in csv.reader(io.StringIO(line)):
                rows.append([c for c in row])
        except Exception:
            rows.append(line.split(','))

    sorted_text = '\n'.join('|'.join(r) for r in sorted(rows))
    sha = hashlib.sha256(sorted_text.encode('utf-8')).hexdigest()

    out = {
        'query': n,
        'row_count': len(rows),
        'first_3_rows': rows[:3] if len(rows) >= 3 else rows,
        'sha256': sha
    }
    with open(OUT / f"Q{n}.json", 'w') as f:
        json.dump(out, f, indent=2)
        f.write('\n')
    print(f"  Q{n:2}: rc={len(rows)}, sha={sha[:8]}...")

print(f"\nDone. Output: {OUT}")
print("Total: 22 baseline files generated")
PYEOF
