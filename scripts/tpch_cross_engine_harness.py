#!/usr/bin/env python3
"""
TPC-H SF=1 Cross-Engine Harness Helper Module (Issue #4018 V312-18).

This module provides reusable helpers for cross-engine TPC-H validation
at scale-factor 1. It is imported at runtime by
``scripts/tpch_sf10_cross_engine_harness.py`` via
``importlib.util.spec_from_file_location`` so that the SF=10 harness can
reuse the SF=1 fixtures, schema, query rewrites and capture logic.

Public surface (importable by the SF=10 harness):
  Constants:
    REPO_ROOT        Path to sqlrustgo repo root
    QUERIES_DIR      Path to canonical q1..q22.sql
    SCHEMA_DDL       CREATE TABLE statements (TPC-H 8 tables, portable types)
    INDEX_DDL        CREATE INDEX statements for sqlite import performance
    TABLES           dict[table_name, list[(col_name, col_type)]]
    SQLITE_REWRITES  dict[qnum -> SQL] rewriting vendor-specific
                     EXTRACT(YEAR FROM x) → strftime('%Y', x) for Q7/Q8/Q9

  Functions:
    setup_sqlite(sf_dir, db_path)
        Import all 8 TPC-H .tbl fixtures from ``sf_dir`` into a fresh
        SQLite database at ``db_path``.

    run_query(conn, sql_text) -> (rows, count, elapsed_seconds)
        Execute ``sql_text`` against an open ``sqlite3`` connection.
        Returns tuple of:
          * rows      list[tuple] — all result rows
          * count     int         — len(rows)
          * elapsed   float       — wall-clock seconds (sqlite3 doesn't
                                    directly expose this; we measure
                                    with time.perf_counter() around the
                                    execute+fetches)
        Uses connection-level timeout so long queries don't hang forever.

    sort_rows(rows) -> list[tuple]
        Canonical-sort result rows: convert to list of tuples, stringify
        each value (so dates/ints match uniformly), then sort by tuple.

    sha256_of_rows(rows) -> str (hex)
        Stable sha256 of the sorted rows (joined by \\n, columns by \\t).
        Deterministic across Python invocations on identical input.

    write_tsv(path, rows)
        Write rows to ``path`` as a tab-separated file, one row per line.

    log(msg)
        Print ``[harness] <msg>`` to stderr (used by SF=10 harness too).

Why this is a separate module from the SF=10 harness?
    SF=10 generates ~10GB of .tbl data; importing + indexing takes
    minutes, and several SF=10 queries blow past in-memory sort budgets.
    Splitting keeps the SF=1 fast-path CI-friendly while letting SF=10
    grow its own knobs (parallel import, per-query timeout, etc.) without
    polluting the SF=1 path.

Exit codes (when invoked as __main__):
    0  setup_sqlite completed and row counts verified
    1  fixture files missing or import failed
"""
from __future__ import annotations

import hashlib
import os
import sqlite3
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Tuple

# ============================================================================
# Constants
# ============================================================================
SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
QUERIES_DIR = REPO_ROOT / "queries"

# TPC-H 8 tables, in canonical order. Column types use portable SQLite types
# (INTEGER / TEXT / NUMERIC). This matches what the .tbl files deliver via
# dbgen's pipe-delimited output (TPC-H spec §4.2).
TABLES: Dict[str, List[Tuple[str, str]]] = {
    "region": [
        ("r_regionkey", "INTEGER"),
        ("r_name", "TEXT"),
        ("r_comment", "TEXT"),
    ],
    "nation": [
        ("n_nationkey", "INTEGER"),
        ("n_name", "TEXT"),
        ("n_regionkey", "INTEGER"),
        ("n_comment", "TEXT"),
    ],
    "supplier": [
        ("s_suppkey", "INTEGER"),
        ("s_name", "TEXT"),
        ("s_address", "TEXT"),
        ("s_nationkey", "INTEGER"),
        ("s_phone", "TEXT"),
        ("s_acctbal", "NUMERIC"),
        ("s_comment", "TEXT"),
    ],
    "customer": [
        ("c_custkey", "INTEGER"),
        ("c_name", "TEXT"),
        ("c_address", "TEXT"),
        ("c_nationkey", "INTEGER"),
        ("c_phone", "TEXT"),
        ("c_acctbal", "NUMERIC"),
        ("c_mktsegment", "TEXT"),
        ("c_comment", "TEXT"),
    ],
    "part": [
        ("p_partkey", "INTEGER"),
        ("p_name", "TEXT"),
        ("p_mfgr", "TEXT"),
        ("p_brand", "TEXT"),
        ("p_type", "TEXT"),
        ("p_size", "INTEGER"),
        ("p_container", "TEXT"),
        ("p_retailprice", "NUMERIC"),
        ("p_comment", "TEXT"),
    ],
    "partsupp": [
        ("ps_partkey", "INTEGER"),
        ("ps_suppkey", "INTEGER"),
        ("ps_availqty", "INTEGER"),
        ("ps_supplycost", "NUMERIC"),
        ("ps_comment", "TEXT"),
    ],
    "orders": [
        ("o_orderkey", "INTEGER"),
        ("o_custkey", "INTEGER"),
        ("o_orderstatus", "TEXT"),
        ("o_totalprice", "NUMERIC"),
        ("o_orderdate", "TEXT"),
        ("o_orderpriority", "TEXT"),
        ("o_clerk", "TEXT"),
        ("o_shippriority", "INTEGER"),
        ("o_comment", "TEXT"),
    ],
    "lineitem": [
        ("l_orderkey", "INTEGER"),
        ("l_partkey", "INTEGER"),
        ("l_suppkey", "INTEGER"),
        ("l_linenumber", "INTEGER"),
        ("l_quantity", "NUMERIC"),
        ("l_extendedprice", "NUMERIC"),
        ("l_discount", "NUMERIC"),
        ("l_tax", "NUMERIC"),
        ("l_returnflag", "TEXT"),
        ("l_linestatus", "TEXT"),
        ("l_shipdate", "TEXT"),
        ("l_commitdate", "TEXT"),
        ("l_receiptdate", "TEXT"),
        ("l_shipinstruct", "TEXT"),
        ("l_shipmode", "TEXT"),
        ("l_comment", "TEXT"),
    ],
}


def _table_ddl(name: str, cols: List[Tuple[str, str]]) -> str:
    return f"CREATE TABLE {name} ({', '.join(f'{n} {t}' for n, t in cols)});"


SCHEMA_DDL = ";\n".join(_table_ddl(n, c) for n, c in TABLES.items()) + ";"


# Indexes that materially speed up SF=1 / SF=10 TPC-H queries. The cross-engine
# comparison only requires functional parity, so we apply the same indexes to
# every engine (Postgres/MySQL/SQLite) — anything missing here would unfairly
# penalize one engine.
INDEX_DDL = (
    "CREATE INDEX idx_lineitem_orderkey ON lineitem(l_orderkey);\n"
    "CREATE INDEX idx_lineitem_partkey ON lineitem(l_partkey);\n"
    "CREATE INDEX idx_lineitem_suppkey ON lineitem(l_suppkey);\n"
    "CREATE INDEX idx_lineitem_shipdate ON lineitem(l_shipdate);\n"
    "CREATE INDEX idx_orders_custkey ON orders(o_custkey);\n"
    "CREATE INDEX idx_orders_orderdate ON orders(o_orderdate);\n"
    "CREATE INDEX idx_customer_nationkey ON customer(c_nationkey);\n"
    "CREATE INDEX idx_partsupp_partkey ON partsupp(ps_partkey);\n"
    "CREATE INDEX idx_partsupp_suppkey ON partsupp(ps_suppkey);\n"
    "CREATE INDEX idx_supplier_nationkey ON supplier(s_nationkey);\n"
    "CREATE INDEX idx_nation_regionkey ON nation(n_regionkey);\n"
)


# SQLite-specific rewrites for the 3 standard TPC-H queries that use
# vendor-specific EXTRACT(YEAR FROM x). The standard q*.sql files are NOT
# modified; this dict provides a SQLite-runnable equivalent. Marked:
# original=standard_tpch, executed_as=sqlite_rewrite.
SQLITE_REWRITES: Dict[int, str] = {
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


# ============================================================================
# Helpers
# ============================================================================
def log(msg: str) -> None:
    """Print a prefixed log line to stderr (SF=10 harness reuses this)."""
    print(f"[harness] {msg}", file=sys.stderr)


def setup_sqlite(sf_dir: Path, db_path: Path) -> None:
    """Import all 8 TPC-H .tbl fixtures from ``sf_dir`` into SQLite ``db_path``.

    Workflow:
      1. Delete ``db_path`` if it exists (fresh start).
      2. Create all 8 tables using TABLES schema.
      3. Apply INDEX_DDL.
      4. For each table, run sqlite3 .import (using subprocess) to load
         the .tbl file. .import requires pipe-delimited input, which is
         exactly what dbgen emits (TPC-H spec §4.2.1).

    Exits (sys.exit 1) if any table is missing or import fails.
    """
    sf_dir = Path(sf_dir)
    db_path = Path(db_path)
    if not sf_dir.is_dir():
        log(f"ERROR: SF dir not found: {sf_dir}")
        sys.exit(1)

    if db_path.exists():
        db_path.unlink()

    conn = sqlite3.connect(str(db_path))
    conn.executescript(SCHEMA_DDL)
    conn.executescript(INDEX_DDL)
    conn.commit()
    conn.close()

    # sqlite3 .import needs to run in an interactive session because .import
    # is a shell-only meta-command. Pipe the .import commands via stdin.
    for table in TABLES:
        tbl_file = sf_dir / f"{table}.tbl"
        if not tbl_file.exists():
            log(f"  [sqlite] SKIP {table}: {tbl_file} not found")
            continue
        # sqlite3 .import cannot have .separator set later than .import,
        # so order matters.
        script = f".mode list\n.separator |\n.import '{tbl_file}' {table}\n"
        try:
            proc = subprocess.run(
                ["sqlite3", str(db_path)],
                input=script,
                capture_output=True,
                text=True,
                timeout=120,
                check=False,
            )
        except subprocess.TimeoutExpired:
            log(f"  [sqlite] import {table}: TIMEOUT after 120s")
            sys.exit(1)
        if proc.returncode != 0:
            log(f"  [sqlite] import {table} FAILED: {proc.stderr.strip()}")
            sys.exit(1)
        # Verify row count > 0 to catch truncated/stub fixtures.
        check_conn = sqlite3.connect(str(db_path))
        try:
            (n,) = check_conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()
        except sqlite3.OperationalError as e:
            log(f"  [sqlite] count {table} FAILED: {e}")
            check_conn.close()
            sys.exit(1)
        check_conn.close()
        log(f"  [sqlite] {table}: {n:,} rows")


def run_query(conn: sqlite3.Connection, sql_text: str) -> Tuple[List[Tuple[Any, ...]], int, float]:
    """Execute ``sql_text`` against an open SQLite connection.

    Returns:
        rows     list of tuples (sqlite3.Row converted to tuple)
        count    len(rows)
        elapsed  float seconds (perf_counter() end - start)
    """
    start = time.perf_counter()
    cur = conn.execute(sql_text)
    rows = cur.fetchall()
    elapsed = time.perf_counter() - start
    return [tuple(r) for r in rows], len(rows), elapsed


def sort_rows(rows: List[Tuple[Any, ...]]) -> List[Tuple[str, ...]]:
    """Canonical-sort rows for deterministic sha256.

    Each value is stringified (so types don't matter for ordering) and
    converted to a tuple. Sorting is lexicographic on tuples.
    """
    return sorted(tuple("" if v is None else str(v) for v in row) for row in rows)


def sha256_of_rows(rows: List[Tuple[str, ...]]) -> str:
    """Deterministic sha256 of sorted rows.

    Rows are joined by ``\\n`` and columns by ``\\t`` so the hash is
    stable across Python invocations on identical input (no platform
    newline or repr differences).
    """
    h = hashlib.sha256()
    for row in rows:
        h.update(("\t".join(row) + "\n").encode("utf-8"))
    return h.hexdigest()


def write_tsv(path: Path, rows: List[Tuple[str, ...]]) -> None:
    """Write rows to ``path`` as a tab-separated file."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        for row in rows:
            fh.write("\t".join("" if v is None else str(v) for v in row) + "\n")


# ============================================================================
# Optional CLI for ad-hoc SF=1 verification
# ============================================================================
def _main() -> int:
    """Entry point when invoked directly:
        python3 scripts/tpch_cross_engine_harness.py --sf-dir /tmp/tpch-sf1
    """
    import argparse

    ap = argparse.ArgumentParser(description="TPC-H SF=1 cross-engine helper (Issue #4018)")
    ap.add_argument("--sf-dir", type=Path, default=Path("/tmp/tpch-sf1"),
                    help="SF=1 .tbl fixture dir (default: /tmp/tpch-sf1)")
    ap.add_argument("--db", type=Path, default=Path("/tmp/tpch_sf1_cross_engine.db"),
                    help="SQLite database path (default: /tmp/tpch_sf1_cross_engine.db)")
    args = ap.parse_args()

    log(f"REPO_ROOT={REPO_ROOT}")
    log(f"QUERIES_DIR={QUERIES_DIR}")
    log(f"SF dir={args.sf_dir}")
    log(f"db={args.db}")
    log(f"queries available: {sum(1 for n in range(1, 23) if (QUERIES_DIR / f'q{n}.sql').exists())}/22")
    log(f"SQLITE_REWRITES keys: {sorted(SQLITE_REWRITES.keys())}")
    setup_sqlite(args.sf_dir, args.db)
    log("DONE")
    return 0


if __name__ == "__main__":
    sys.exit(_main())
