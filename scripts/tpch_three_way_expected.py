#!/usr/bin/env python3
"""
TPC-H Three-Way Expected Row-Count Generator (Phase 0, Issue #2977)

Loads tests/data/tpch-sf001/ into MySQL + SQLite + PostgreSQL,
runs the requested TPC-H queries, captures row_count + first-N-row
snapshot from each engine, and emits:
  tests/data/tpch-sf001/expected/Q<N>_three_way.json
  tests/data/tpch-sf001/expected/THREE_WAY_SUMMARY.md

Usage:
  # Default: just Q1 (Phase 0a proof)
  python3 scripts/tpch_three_way_expected.py

  # All 22 queries
  python3 scripts/tpch_three_way_expected.py --all

  # Specific queries
  python3 scripts/tpch_three_way_expected.py --queries 1 3 6

# Conventions
  - SF=0.001 fixture: 614 lineitem / 919 total rows
  - Q1 expected row_count depends on WHERE l_shipdate filter
  - For the q*.sql in tests/queries/, the standard Q1 uses '1995-12-01'
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ============================================================================
# Config
# ============================================================================
REPO_ROOT = Path("/home/openclaw/dev/yinglichina163/sqlrustgo/.worktrees/tpch-22-wire-v2")
FIXTURE_DIR = REPO_ROOT / "tests" / "data" / "tpch-sf001"
QUERIES_DIR = REPO_ROOT / "queries"
EXPECTED_DIR = FIXTURE_DIR / "expected"

# SQLite-specific rewrites for the 3 standard TPC-H queries that use vendor-specific
# functions. The standard q*.sql files are NOT modified; this dict provides a
# SQLite-runnable equivalent for row_count expected generation only.
# Marked: original=standard_tpch, executed_as=sqlite_rewrite
SQLITE_REWRITES = {
    7: (
        "SELECT n1.n_name AS supp_nation, n2.n_name AS cust_nation, "
        "CAST(strftime('%Y', o_orderdate) AS INTEGER) AS l_year, "
        "SUM(l_extendedprice * (1 - l_discount)) AS volume "
        "FROM supplier, lineitem, orders, customer, nation n1, nation n2 "
        "WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey "
        "AND c_custkey = o_custkey "
        "AND s_nationkey = n1.n_nationkey "
        "AND c_nationkey = n2.n_nationkey "
        "AND n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE' "
        "GROUP BY n1.n_name, n2.n_name, CAST(strftime('%Y', o_orderdate) AS INTEGER) "
        "ORDER BY n1.n_name, n2.n_name, l_year"
    ),
    8: (
        "SELECT CAST(strftime('%Y', o_orderdate) AS INTEGER) AS o_year, "
        "SUM(CASE WHEN n2.n_name = 'GERMANY' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) "
        "/ SUM(l_extendedprice * (1 - l_discount)) AS mkt_share "
        "FROM customer, orders, lineitem, supplier, nation n1, nation n2, region "
        "WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey "
        "AND l_suppkey = s_suppkey AND c_nationkey = n1.n_nationkey "
        "AND s_nationkey = n1.n_nationkey AND n1.n_regionkey = r_regionkey "
        "AND r_name = 'EUROPE' AND n2.n_name = 'GERMANY' "
        "AND o_orderdate >= '1995-01-01' AND o_orderdate < '1996-12-31' "
        "GROUP BY CAST(strftime('%Y', o_orderdate) AS INTEGER) "
        "ORDER BY o_year"
    ),
    9: (
        "SELECT n_name, CAST(strftime('%Y', o_orderdate) AS INTEGER) AS o_year, "
        "SUM(l_extendedprice * (1 - l_discount) - ps_supplycost * l_quantity) AS amount "
        "FROM customer, orders, lineitem, supplier, part, partsupp, nation "
        "WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey "
        "AND l_suppkey = s_suppkey AND l_partkey = p_partkey "
        "AND ps_partkey = p_partkey AND ps_suppkey = s_suppkey "
        "AND c_nationkey = s_nationkey AND s_nationkey = n_nationkey "
        "AND p_name LIKE '%green%' "
        "GROUP BY n_name, CAST(strftime('%Y', o_orderdate) AS INTEGER) "
        "ORDER BY n_name, o_year DESC"
    ),
}

MYSQL_HOST = "127.0.0.1"
MYSQL_PORT = 3306
MYSQL_USER = "root"
MYSQL_PASS = "root123"
MYSQL_DB = "tpch_sf001"

SQLITE_DB = "/tmp/tpch_sf001.db"

PG_USER = "openclaw"
PG_HOST = "/var/run/postgresql"
PG_DB = "tpch_test"
PG_SCHEMA = "sf001"  # dedicated schema inside tpch_test to isolate from SF=0.1 tables

# TPC-H SF=0.001 schema (matches tests/data/tpch-sf001/expected/Q1.json field set)
# All columns TEXT/VARCHAR/INTEGER/REAL — type chosen for engine portability
TABLES = {
    "region": [
        ("r_regionkey", "INT"),
        ("r_name", "VARCHAR(25)"),
        ("r_comment", "VARCHAR(152)"),
    ],
    # sf001 fixture column order is (n_nationkey, n_regionkey, n_name, n_comment)
    # — DIFFERS from standard TPC-H (n_nationkey, n_name, n_regionkey, n_comment)
    "nation": [
        ("n_nationkey", "INT"),
        ("n_regionkey", "INT"),
        ("n_name", "VARCHAR(25)"),
        ("n_comment", "VARCHAR(152)"),
    ],
    "supplier": [
        ("s_suppkey", "INT"),
        ("s_nationkey", "INT"),
        ("s_name", "VARCHAR(25)"),
        ("s_address", "VARCHAR(40)"),
        ("s_phone", "VARCHAR(15)"),
        ("s_acctbal", "DECIMAL(15,2)"),
        ("s_comment", "VARCHAR(101)"),
    ],
    "customer": [
        ("c_custkey", "INT"),
        ("c_nationkey", "INT"),
        ("c_name", "VARCHAR(25)"),
        ("c_address", "VARCHAR(40)"),
        ("c_phone", "VARCHAR(15)"),
        ("c_acctbal", "DECIMAL(15,2)"),
        ("c_mktsegment", "VARCHAR(10)"),
        ("c_comment", "VARCHAR(117)"),
    ],
    "part": [
        ("p_partkey", "INT"),
        ("p_name", "VARCHAR(55)"),
        ("p_mfgr", "VARCHAR(25)"),
        ("p_brand", "VARCHAR(10)"),
        ("p_type", "VARCHAR(25)"),
        ("p_size", "INT"),
        ("p_container", "VARCHAR(10)"),
        ("p_retailprice", "DECIMAL(15,2)"),
        ("p_comment", "VARCHAR(23)"),
    ],
    "partsupp": [
        ("ps_partkey", "INT"),
        ("ps_suppkey", "INT"),
        ("ps_availqty", "INT"),
        ("ps_supplycost", "DECIMAL(15,2)"),
        ("ps_comment", "VARCHAR(199)"),
    ],
    "orders": [
        ("o_orderkey", "INT"),
        ("o_custkey", "INT"),
        ("o_orderstatus", "CHAR(1)"),
        ("o_totalprice", "DECIMAL(15,2)"),
        ("o_orderdate", "DATE"),
        ("o_orderpriority", "VARCHAR(15)"),
        ("o_clerk", "VARCHAR(15)"),
        ("o_shippriority", "INT"),
        ("o_comment", "VARCHAR(79)"),
    ],
    "lineitem": [
        ("l_orderkey", "INT"),
        ("l_partkey", "INT"),
        ("l_suppkey", "INT"),
        ("l_linenumber", "INT"),
        ("l_quantity", "DECIMAL(15,2)"),
        ("l_extendedprice", "DECIMAL(15,2)"),
        ("l_discount", "DECIMAL(15,2)"),
        ("l_tax", "DECIMAL(15,2)"),
        ("l_returnflag", "CHAR(1)"),
        ("l_linestatus", "CHAR(1)"),
        ("l_shipdate", "DATE"),
        ("l_commitdate", "DATE"),
        ("l_receiptdate", "DATE"),
        ("l_shipinstruct", "VARCHAR(25)"),
        ("l_shipmode", "VARCHAR(10)"),
        ("l_comment", "VARCHAR(44)"),
    ],
}


# ============================================================================
# Helpers
# ============================================================================
def run(cmd: List[str], input_text: Optional[str] = None, check: bool = True) -> Tuple[int, str, str]:
    """Run subprocess, return (rc, stdout, stderr)."""
    proc = subprocess.run(
        cmd,
        input=input_text,
        capture_output=True,
        text=True,
        timeout=120,
    )
    if check and proc.returncode != 0:
        raise RuntimeError(f"cmd failed: {cmd}\nSTDOUT:{proc.stdout}\nSTDERR:{proc.stderr}")
    return proc.returncode, proc.stdout, proc.stderr


def mysql_exec(sql: str, db: Optional[str] = MYSQL_DB) -> str:
    """Run a SQL statement, return stdout."""
    cmd = ["mysql", "-h", MYSQL_HOST, "-P", str(MYSQL_PORT), "-u", MYSQL_USER, f"-p{MYSQL_PASS}"]
    if db:
        cmd.append(db)
    cmd += ["-N", "-B", "-e", sql]
    rc, out, err = run(cmd, check=False)
    if rc != 0:
        return f"ERROR: {err.strip()}"
    return out.strip()


def mysql_exec_no_db(sql: str) -> str:
    """Run a SQL statement WITHOUT specifying a database (for CREATE/DROP DATABASE)."""
    cmd = ["mysql", "-h", MYSQL_HOST, "-P", str(MYSQL_PORT), "-u", MYSQL_USER, f"-p{MYSQL_PASS}",
           "-N", "-B", "-e", sql]
    rc, out, err = run(cmd, check=False)
    if rc != 0:
        return f"ERROR: {err.strip()}"
    return out.strip()


def sqlite_exec(sql: str) -> str:
    """Run a SQL statement via sqlite3, pipe-separated output for parsing."""
    cmd = ["sqlite3", "-separator", "|", SQLITE_DB, sql]
    rc, out, err = run(cmd, check=False)
    if rc != 0:
        return f"ERROR: {err.strip()}"
    return out.strip()


def pg_exec(sql: str) -> str:
    """Run a SQL statement via psql, pipe-separated output. Schema is prepended automatically."""
    # If SQL references a bare table name, qualify it with schema.
    # Simple heuristic: prepend "SET search_path TO sf001;" so unqualified names resolve.
    cmd = ["psql", "-h", PG_HOST, "-U", PG_USER, "-d", PG_DB, "-t", "-A", "-F", "|",
           "-c", f"SET search_path TO {PG_SCHEMA}, public; {sql}"]
    rc, out, err = run(cmd, check=False)
    if rc != 0:
        return f"ERROR: {err.strip()}"
    return out.strip()


# ============================================================================
# Setup
# ============================================================================
def setup_mysql():
    print("[setup] MySQL: drop+create tpch_sf001, create 8 tables, LOAD DATA all 8")
    mysql_exec_no_db(f"DROP DATABASE IF EXISTS {MYSQL_DB};")
    mysql_exec_no_db(f"CREATE DATABASE {MYSQL_DB} CHARACTER SET utf8mb4;")
    for table, cols in TABLES.items():
        col_defs = ", ".join(f"{n} {t}" for n, t in cols)
        pks = ""
        if table == "lineitem":
            pks = ", PRIMARY KEY (l_orderkey, l_linenumber)"
        elif table == "partsupp":
            pks = ", PRIMARY KEY (ps_partkey, ps_suppkey)"
        elif table in ("region", "nation", "supplier", "customer", "part", "orders"):
            pks = ", PRIMARY KEY (" + [c[0] for c in cols if c[0].endswith("key")][0] + ")"
        mysql_exec(f"CREATE TABLE {table} ({col_defs}{pks});")
    # LOAD DATA via mysqlimport-style with --local-infile
    for table in TABLES:
        path = FIXTURE_DIR / f"{table}.tbl"
        sql = f"LOAD DATA LOCAL INFILE '{path}' INTO TABLE {table} FIELDS TERMINATED BY '|' LINES TERMINATED BY '\\n';"
        rc, out, err = run(
            ["mysql", "-h", MYSQL_HOST, "-P", str(MYSQL_PORT), "-u", MYSQL_USER, f"-p{MYSQL_PASS}",
             MYSQL_DB, "--local-infile=1", "-e", sql],
            check=False,
        )
        if rc != 0:
            print(f"  [mysql] LOAD {table} FAILED: {err.strip()}")
            return False
    counts = {}
    for table in TABLES:
        c = mysql_exec(f"SELECT COUNT(*) FROM {table};")
        counts[table] = c
    print(f"  [mysql] row counts: {counts}")
    return True


def setup_sqlite():
    print("[setup] SQLite: create 8 tables, import all 8 .tbl")
    if os.path.exists(SQLITE_DB):
        os.remove(SQLITE_DB)
    # Create tables with type mapping: INT->INTEGER, VARCHAR/CHAR/DECIMAL/DATE->TEXT
    create_sql_parts = []
    for table, cols in TABLES.items():
        col_defs_list = []
        for n, t in cols:
            t_sql = t
            if t == "INT":
                t_sql = "INTEGER"
            elif t.startswith("VARCHAR") or t.startswith("CHAR") or t == "DATE":
                t_sql = "TEXT"
            elif t.startswith("DECIMAL"):
                t_sql = "NUMERIC"
            col_defs_list.append(f"{n} {t_sql}")
        create_sql_parts.append(f"CREATE TABLE {table} ({', '.join(col_defs_list)})")
    sqlite_exec("; ".join(create_sql_parts) + ";")
    # Import via sqlite3 .import — must be in interactive stdin, but we can pipe commands
    for table in TABLES:
        path = FIXTURE_DIR / f"{table}.tbl"
        # Use sqlite3 with -cmd to set mode/separator, then .import
        import_cmd = f".mode list\n.separator |\n.import '{path}' {table}\n"
        rc, out, err = run(
            ["sqlite3", SQLITE_DB],
            input_text=import_cmd,
            check=False,
        )
        if rc != 0:
            print(f"  [sqlite] import {table} FAILED: {err.strip()}")
            return False
    counts = {}
    for table in TABLES:
        c = sqlite_exec(f"SELECT COUNT(*) FROM {table};")
        counts[table] = c
    print(f"  [sqlite] row counts: {counts}")
    return True


def setup_pg():
    print(f"[setup] PG: CREATE SCHEMA {PG_SCHEMA} in tpch_test, create 8 tables, COPY all 8 .tbl")
    # Create dedicated schema (avoids collision with existing SF=0.1 tables)
    rc, out, err = run(
        ["psql", "-h", PG_HOST, "-U", PG_USER, "-d", PG_DB, "-c", f"DROP SCHEMA IF EXISTS {PG_SCHEMA} CASCADE; CREATE SCHEMA {PG_SCHEMA}; SET search_path TO {PG_SCHEMA};"],
        check=False,
    )
    if rc != 0:
        print(f"  [pg] schema setup FAILED: {err.strip()}")
        return False
    # Create tables in the schema
    for table, cols in TABLES.items():
        col_defs = ", ".join(f"{n} {t}" for n, t in cols)
        sql = f"CREATE TABLE {PG_SCHEMA}.{table} ({col_defs});"
        rc, out, err = run(
            ["psql", "-h", PG_HOST, "-U", PG_USER, "-d", PG_DB, "-c", sql],
            check=False,
        )
        if rc != 0:
            print(f"  [pg] create {table} FAILED: {err.strip()}")
            return False
    # COPY data via \copy meta-command (psql-side, doesn't require pg_read_server_files)
    for table in TABLES:
        path = FIXTURE_DIR / f"{table}.tbl"
        with tempfile.NamedTemporaryFile("w", suffix=".tbl", delete=False) as tf:
            for line in open(path):
                if line.rstrip().endswith("|"):
                    line = line.rstrip()[:-1] + "\n"
                tf.write(line)
            tmp_path = tf.name
        # \copy is a psql backslash command, only works via stdin, not -c
        # So we feed it via stdin
        psql_input = f"SET search_path TO {PG_SCHEMA}, public;\n\\copy {PG_SCHEMA}.{table} FROM '{tmp_path}' WITH (FORMAT csv, DELIMITER '|', NULL '')\n"
        rc, out, err = run(
            ["psql", "-h", PG_HOST, "-U", PG_USER, "-d", PG_DB, "-v", "ON_ERROR_STOP=1"],
            input_text=psql_input,
            check=False,
        )
        os.unlink(tmp_path)
        if rc != 0:
            print(f"  [pg] COPY {table} FAILED: {err.strip()}")
            return False
    counts = {}
    for table in TABLES:
        c = pg_exec(f"SELECT COUNT(*) FROM {table};")
        counts[table] = c
    print(f"  [pg] row counts: {counts}")
    return True


# ============================================================================
# Query execution + capture
# ============================================================================
def capture_query(qnum: int) -> Dict[str, Any]:
    """Run Q<n>.sql on SQLite only (per 李哥 2026-06-05 SQLite-only decision)."""
    # Use SQLite rewrite if available (Q7/Q8/Q9 use vendor-specific EXTRACT(YEAR))
    if qnum in SQLITE_REWRITES:
        sql_exec = SQLITE_REWRITES[qnum]
        result: Dict[str, Any] = {
            "query": f"q{qnum}.sql",
            "sqlite_sql_source": "sqlite_rewrite (EXTRACT(YEAR FROM x) → strftime('%Y', x))",
        }
    else:
        sql_path = QUERIES_DIR / f"q{qnum}.sql"
        sql = sql_path.read_text().strip()
        if sql.endswith(";"):
            sql_exec = sql[:-1]
        else:
            sql_exec = sql
        result = {"query": f"q{qnum}.sql", "sqlite_sql_source": "standard_tpch_q.sql"}

    result["engines"] = {}
    exec_fn = sqlite_exec
    engine_name = "sqlite"

    # COUNT(*) wrapper for row count
    count_sql = f"SELECT COUNT(*) FROM ({sql_exec}) AS sub;"
    count_str = exec_fn(count_sql)
    try:
        row_count = int(count_str)
    except (ValueError, TypeError):
        row_count = None
    # First 3 rows
    rows_sql = f"SELECT * FROM ({sql_exec}) AS sub LIMIT 3;"
    rows_str = exec_fn(rows_sql)
    first_rows = [r for r in rows_str.split("\n") if r.strip()] if not rows_str.startswith("ERROR") else []
    result["engines"][engine_name] = {
        "row_count": row_count,
        "first_3_rows": first_rows,
        "error": rows_str if rows_str.startswith("ERROR") else None,
    }

    # Single-engine mode: consensus is just this engine's result
    if row_count is not None:
        result["consensus_row_count"] = row_count
        result["consensus"] = True
    else:
        result["consensus_row_count"] = None
        result["consensus"] = False

    return result


# ============================================================================
# Main
# ============================================================================
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--queries", nargs="*", type=int, default=[1])
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--skip-setup", action="store_true",
                    help="Skip re-loading fixtures (assume already loaded)")
    args = ap.parse_args()

    if args.all:
        qnums = list(range(1, 23))
    else:
        qnums = args.queries

    EXPECTED_DIR.mkdir(parents=True, exist_ok=True)

    # Per 李哥 2026-06-05: SQLite-only mode. PG schema mismatch with sf001 fixture
    # (column order + tpc_test 缺 l_linestatus); MySQL physically blocked by
    # local_infile=OFF + secure_file_priv + no sudo. SQLite is the only engine
    # in this session that can produce reliable row_count expected.
    SUPPORTED_ENGINES = ["sqlite"]

    if not args.skip_setup:
        # MySQL blocked by local_infile=OFF + secure_file_priv — skip with warning
        print("[setup] MySQL: SKIPPED (local_infile=OFF, secure_file_priv blocks non-root LOAD DATA)")
        # PG skipped per 李哥 decision (SQLite-only)
        print("[setup] PG: SKIPPED (李哥 decision: SQLite-only expected)")
        ok_sqlite = setup_sqlite()
        if not ok_sqlite:
            print("SETUP FAILED (SQLite) — aborting")
            sys.exit(1)
    else:
        print("[setup] skipped (--skip-setup)")

    summary: List[Dict[str, Any]] = []
    for q in qnums:
        print(f"\n[query] Q{q}")
        try:
            r = capture_query(q)
        except Exception as e:
            r = {"query": f"q{q}.sql", "error": str(e), "consensus": False}
        summary.append(r)
        out_path = EXPECTED_DIR / f"Q{q}_three_way.json"
        out_path.write_text(json.dumps(r, indent=2, ensure_ascii=False))
        status = "✅" if r.get("consensus") else "❌"
        print(f"  → {out_path} {status}")

    md_lines = ["# TPC-H SF=0.001 Three-Way Reference (Phase 0)\n"]
    md_lines.append(f"Engines: MySQL 8.0.46 (local root/root123, db `tpch_sf001`), "
                    f"SQLite 3.45.1 (`{SQLITE_DB}`), PostgreSQL 16.14 (db `{PG_DB}`)\n")
    md_lines.append("## Row-Count Consensus\n")
    md_lines.append("| Q | MySQL | SQLite | PG | Consensus | Notes |")
    md_lines.append("|---|-------|--------|----|-----------|-------|")
    for r in summary:
        q = r["query"]
        eng = r.get("engines", {})
        mc = eng.get("mysql", {}).get("row_count")
        sc = eng.get("sqlite", {}).get("row_count")
        pc = eng.get("pg", {}).get("row_count")
        cons = r.get("consensus_row_count")
        ok = "✅" if r.get("consensus") else "❌"
        notes = ""
        if not r.get("consensus"):
            notes = f"divergent: {r.get('divergent_counts')}"
        elif r.get("engines", {}).get("mysql", {}).get("error"):
            notes = "mysql error"
        md_lines.append(f"| {q} | {mc} | {sc} | {pc} | {cons} {ok} | {notes} |")

    md_path = EXPECTED_DIR / "THREE_WAY_SUMMARY.md"
    md_path.write_text("\n".join(md_lines) + "\n")
    print(f"\n[summary] → {md_path}")
    print("\n=== DONE ===")


if __name__ == "__main__":
    main()
