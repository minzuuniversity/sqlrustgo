#!/usr/bin/env python3
"""Build the SQLite ground-truth database consumed by tests/tpch_q9_audit.rs.

The TPC-H 22 in-process audit (`tests/tpch_q9_audit.rs`) compares each
query's `SELECT COUNT(*) FROM (...)` result against a SQLite baseline.
The baseline database is loaded from the canonical SF=0.01 `.tbl` files
under `tests/data/tpch-sf01/` (lineitem ≈ 60 000 rows).

Why this script exists
----------------------
The original audit pointed at `/tmp/tpch_3way_sf001.db` (one-shot Sprint 5
v8 staging).  When that directory was cleaned up the SQLite db went with
it.  This script regenerates the baseline from the versioned `.tbl`
files so the audit is reproducible on any developer machine and CI
runner.

Usage
-----

    python3 scripts/dev/build_tpch_sf01_sqlite.py

By default the database is written to ``/tmp/tpch_sf01_audit.db`` which
matches the constant ``SQLITE_BASELINE_DB`` in ``tests/tpch_q9_audit.rs``.
Override via the ``TPCH_SF01_SQLITE_DB`` environment variable.
"""
from __future__ import annotations

import os
import sqlite3
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_DIR = REPO_ROOT / "tests" / "data" / "tpch-sf01"
DEFAULT_DB_PATH = "/tmp/tpch_sf01_audit.db"

TABLES: list[tuple[str, list[str]]] = [
    ("region", [
        "r_regionkey INTEGER", "r_name TEXT", "r_comment TEXT",
    ]),
    ("nation", [
        "n_nationkey INTEGER", "n_name TEXT", "n_regionkey INTEGER",
        "n_comment TEXT",
    ]),
    ("supplier", [
        "s_suppkey INTEGER", "s_name TEXT", "s_address TEXT",
        "s_nationkey INTEGER", "s_phone TEXT", "s_acctbal REAL",
        "s_comment TEXT",
    ]),
    ("customer", [
        "c_custkey INTEGER", "c_name TEXT", "c_address TEXT",
        "c_nationkey INTEGER", "c_phone TEXT", "c_acctbal REAL",
        "c_mktsegment TEXT", "c_comment TEXT",
    ]),
    ("part", [
        "p_partkey INTEGER", "p_name TEXT", "p_mfgr TEXT", "p_brand TEXT",
        "p_type TEXT", "p_size INTEGER", "p_container TEXT",
        "p_retailprice REAL", "p_comment TEXT",
    ]),
    ("partsupp", [
        "ps_partkey INTEGER", "ps_suppkey INTEGER", "ps_availqty INTEGER",
        "ps_supplycost REAL", "ps_comment TEXT",
    ]),
    ("orders", [
        "o_orderkey INTEGER", "o_custkey INTEGER", "o_orderstatus TEXT",
        "o_totalprice REAL", "o_orderdate TEXT", "o_orderpriority TEXT",
        "o_clerk TEXT", "o_shippriority INTEGER", "o_comment TEXT",
    ]),
    ("lineitem", [
        "l_orderkey INTEGER", "l_partkey INTEGER", "l_suppkey INTEGER",
        "l_linenumber INTEGER", "l_quantity REAL", "l_extendedprice REAL",
        "l_discount REAL", "l_tax REAL", "l_returnflag TEXT",
        "l_linestatus TEXT", "l_shipdate TEXT", "l_commitdate TEXT",
        "l_receiptdate TEXT", "l_shipinstruct TEXT", "l_shipmode TEXT",
        "l_comment TEXT",
    ]),
]


def load_table(conn: sqlite3.Connection, name: str, cols: list[str],
               tbl_path: Path) -> int:
    conn.execute(f"DROP TABLE IF EXISTS {name}")
    conn.execute(f"CREATE TABLE {name} ({', '.join(cols)})")
    rows: list[list[str]] = []
    with tbl_path.open() as fh:
        for line in fh:
            line = line.rstrip("\n")
            if line.endswith("|"):
                line = line[:-1]
            if not line:
                continue
            rows.append(line.split("|"))
    placeholders = ",".join("?" for _ in cols)
    conn.executemany(f"INSERT INTO {name} VALUES ({placeholders})", rows)
    conn.commit()
    return len(rows)


def main() -> int:
    fixture_dir = Path(os.environ.get("TPCH_SF01_FIXTURE_DIR",
                                      DEFAULT_FIXTURE_DIR))
    db_path = Path(os.environ.get("TPCH_SF01_SQLITE_DB", DEFAULT_DB_PATH))
    if not fixture_dir.is_dir():
        print(f"error: fixture dir not found: {fixture_dir}", file=sys.stderr)
        return 1

    if db_path.exists():
        db_path.unlink()
    db_path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(db_path))

    print(f"Loading SF=0.01 .tbl files from {fixture_dir}")
    print(f"  -> {db_path}")
    expected_total = {
        "region": 5, "nation": 25, "supplier": 100, "customer": 1500,
        "part": 2000, "partsupp": 8000, "orders": 15000, "lineitem": 60000,
    }
    for name, cols in TABLES:
        tbl_path = fixture_dir / f"{name}.tbl"
        if not tbl_path.exists():
            print(f"error: missing {tbl_path}", file=sys.stderr)
            return 1
        n = load_table(conn, name, cols, tbl_path)
        marker = "" if n == expected_total[name] else "  <-- UNEXPECTED"
        print(f"  {name:<10} {n:>7,} rows{marker}")
    conn.close()
    print("done.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())