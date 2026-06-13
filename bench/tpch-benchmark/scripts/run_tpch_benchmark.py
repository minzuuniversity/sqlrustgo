#!/usr/bin/env python3
"""
TPC-H Multi-Engine Benchmark Runner
===================================
Supports: SQLite, PostgreSQL, MySQL (MariaDB), SQLRustGo (wire)
Scales:   SF=0.001, SF=0.01, SF=1

Usage:
    python3 run_tpch_benchmark.py [--sf 0.01] [--engines sqlite,pg,mysql,sqlrustgo] [--data-dir /path]
    python3 run_tpch_benchmark.py --capture-baseline   # generates baseline JSON
    python3 run_tpch_benchmark.py --verify            # compare against baseline
"""

import subprocess
import time
import json
import os
import sys
import re
import argparse
import statistics
from pathlib import Path
from typing import Optional

# ── Config ────────────────────────────────────────────────────────────────────
MYSQL_PORT = 3307
SQLRUSTGO_PORT = 3308
TPCH_QUERIES_DIR = Path(__file__).parent.parent / "queries"

# TPC-H row counts at each scale factor
LINEITEM_COUNTS = {
    "sf0.001": 501,
    "sf0.01":  60_000,
    "sf1":     6_001_215,
}

# ── Helpers ───────────────────────────────────────────────────────────────────

def run_cmd(cmd: list[str], timeout=120, capture=True) -> tuple[int, str, str]:
    """Run a shell command, return (returncode, stdout, stderr)."""
    try:
        r = subprocess.run(cmd, capture_output=capture, text=True, timeout=timeout)
        return r.returncode, r.stdout or "", r.stderr or ""
    except subprocess.TimeoutExpired:
        return -1, "", f"timeout after {timeout}s"


def load_sqlite(db_path: str, tbl_dir: str, schema_sql: str) -> bool:
    """Load TPC-H .tbl files into SQLite.
    
    Uses a file-based DB so state persists across commands.
    TPC-H .tbl files use | as delimiter.
    """
    if Path(db_path).exists():
        os.remove(db_path)
    # Write schema to temp file
    schema_file = "/tmp/tpch_sqlite_schema.sql"
    Path(schema_file).write_text(schema_sql)
    run_cmd(["sqlite3", db_path, f".read {schema_file}"])
    for tbl in ["region","nation","supplier","customer","part","partsupp","orders","lineitem"]:
        tbl_path = f"{tbl_dir}/{tbl}.tbl"
        if not Path(tbl_path).exists():
            print(f"  [sqlite] {tbl}.tbl not found, skipping")
            continue
        # Use .separator | then .import (stdin for multi-command)
        dot_cmds = f".separator |\n.import '{tbl_path}' {tbl}\n"
        r = subprocess.run(
            ["sqlite3", db_path],
            input=dot_cmds, text=True, capture_output=True
        )
        if r.returncode != 0:
            print(f"  [sqlite] .import {tbl}: {r.stderr[:120]}")
    return True


def load_pg(db_url: str, tbl_dir: str, schema_sql: str) -> bool:
    """Load TPC-H .tbl files into PostgreSQL."""
    run_cmd(["psql", db_url, "-f", schema_sql])
    for tbl in ["region","nation","supplier","customer","part","partsupp","orders","lineitem"]:
        tbl_path = f"{tbl_dir}/{tbl}.tbl"
        if not Path(tbl_path).exists():
            continue
        # Use COPY FROM STDIN with | delimiter
        with open(tbl_path) as f:
            proc = subprocess.Popen(
                ["psql", db_url, "-c", f"COPY {tbl} FROM STDIN WITH (FORMAT CSV, DELIMITER '|')"],
                stdin=subprocess.PIPE, text=True
            )
            proc.communicate(input=f.read())
            if proc.returncode != 0:
                print(f"  [pg] COPY {tbl} failed")
    return True


def load_mysql(host: str, port: int, db_name: str, tbl_dir: str, schema_sql: str) -> bool:
    """Load TPC-H .tbl files into MySQL/MariaDB.
    
    Workaround for macOS MariaDB client 12.3 bug: 'USE <db>' in -e context
    triggers empty-user re-auth, losing all privileges. Solution: write
    all SQL (DDL + LOAD) to a single .sql file and pipe via stdin.
    """
    # Build a single SQL script with DDL + LOAD statements
    sql_lines = [f"DROP DATABASE IF EXISTS {db_name};",
                 f"CREATE DATABASE {db_name};",
                 f"USE {db_name};"]
    sql_lines.append(schema_sql)
    for tbl in ["region","nation","supplier","customer","part","partsupp","orders","lineitem"]:
        tbl_path = f"{tbl_dir}/{tbl}.tbl"
        if not Path(tbl_path).exists():
            continue
        # LOAD DATA LOCAL INFILE needs full path
        sql_lines.append(
            f"LOAD DATA LOCAL INFILE '{tbl_path}' INTO TABLE {tbl} "
            f"FIELDS TERMINATED BY '|' LINES TERMINATED BY '|';"
        )

    script_path = "/tmp/tpch_mysql_load.sql"
    Path(script_path).write_text("\n".join(sql_lines))

    # Pipe the script via stdin
    r = subprocess.run(
        ["mysql", "-h", host, "-P", str(port), "-u", "liying",
         "--local-infile=1"],
        input=Path(script_path).read_text(),
        capture_output=True, text=True, timeout=300
    )
    if r.returncode != 0:
        print(f"  [mysql] load script failed: {r.stderr[:200]}")
        return False
    return True


def load_sqlrustgo(host: str, port: int, db_name: str, tbl_dir: str) -> bool:
    """Load TPC-H .tbl files into SQLRustGo via mysql CLI stdin.
    
    SQLRustGo needs explicit CREATE TABLE DDLs. Uses stdin to avoid the
    macOS MariaDB client 'USE in -e' bug.
    """
    ddls = [
        "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
        "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
        "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
        "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
        "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
        "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
        "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
    ]

    sql_lines = [f"DROP DATABASE IF EXISTS {db_name};",
                 f"CREATE DATABASE {db_name};",
                 f"USE {db_name};"]
    sql_lines.extend(ddls)
    for tbl in ["region","nation","supplier","customer","part","partsupp","orders","lineitem"]:
        tbl_path = f"{tbl_dir}/{tbl}.tbl"
        if not Path(tbl_path).exists():
            continue
        sql_lines.append(
            f"LOAD DATA LOCAL INFILE '{tbl_path}' INTO TABLE {tbl} "
            f"FIELDS TERMINATED BY '|' LINES TERMINATED BY '|';"
        )

    script_path = "/tmp/tpch_sqlrustgo_load.sql"
    Path(script_path).write_text("\n".join(sql_lines))

    r = subprocess.run(
        ["mysql", "-h", host, "-P", str(port), "-u", "liying",
         "--local-infile=1"],
        input=Path(script_path).read_text(),
        capture_output=True, text=True, timeout=300
    )
    if r.returncode != 0:
        print(f"  [sqlrustgo] load script failed: {r.stderr[:200]}")
        return False
    return True


def read_query(qnum: int) -> str:
    """Read TPC-H query SQL from queries/ directory."""
    qfile = TPCH_QUERIES_DIR / f"q{qnum}.sql"
    if not qfile.exists():
        return ""
    content = qfile.read_text()
    # Strip comments and normalize whitespace
    lines = [line for line in content.splitlines() if not line.strip().startswith("--")]
    return " ".join(lines).strip()


def run_query_sqlite(db_path: str, sql: str) -> tuple[int, float, list[list]]:
    """Run SQL on SQLite. Returns (row_count, duration_ms, sample_rows)."""
    # Wrap in COUNT for timing, then run original for sample
    start = time.perf_counter()
    rc, out, err = run_cmd(["sqlite3", "-header", "-column", db_path, sql], timeout=300)
    elapsed_ms = (time.perf_counter() - start) * 1000
    if rc != 0:
        return -1, elapsed_ms, []
    rows = [line.split("|") for line in out.strip().splitlines() if line.strip()]
    return len(rows), elapsed_ms, rows[:3]


def run_query_pg(db_url: str, sql: str) -> tuple[int, float, list[list]]:
    start = time.perf_counter()
    rc, out, err = run_cmd(
        ["psql", db_url, "--no-align", "--field-separator=|", "--tuples-only", "-c", sql],
        timeout=300
    )
    elapsed_ms = (time.perf_counter() - start) * 1000
    if rc != 0:
        return -1, elapsed_ms, []
    rows = [line.split("|") for line in out.strip().splitlines() if line.strip()]
    return len(rows), elapsed_ms, rows[:3]


def run_query_mysql(host: str, port: int, db: str, sql: str) -> tuple[int, float, list[list]]:
    """Run SQL on MySQL/MariaDB via stdin (avoids 'USE in -e' client bug)."""
    start = time.perf_counter()
    wrapped_sql = f"USE {db};\n{sql.rstrip(';')}"
    r = subprocess.run(
        ["mysql", "-h", host, "-P", str(port), "-u", "liying", "-N"],
        input=wrapped_sql, text=True, capture_output=True, timeout=300
    )
    elapsed_ms = (time.perf_counter() - start) * 1000
    if r.returncode != 0:
        return -1, elapsed_ms, []
    rows = [line.split("\t") for line in r.stdout.strip().splitlines() if line.strip()]
    return len(rows), elapsed_ms, rows[:3]


def run_query_sqlrustgo(host: str, port: int, db: str, sql: str) -> tuple[int, float, list[list]]:
    """Run SQL on SQLRustGo via mysql CLI stdin."""
    start = time.perf_counter()
    wrapped_sql = f"USE {db};\n{sql.rstrip(';')}"
    r = subprocess.run(
        ["mysql", "-h", host, "-P", str(port), "-u", "liying", "-N"],
        input=wrapped_sql, text=True, capture_output=True, timeout=300
    )
    elapsed_ms = (time.perf_counter() - start) * 1000
    if r.returncode != 0:
        return -1, elapsed_ms, []
    rows = [line.split("\t") for line in r.stdout.strip().splitlines() if line.strip()]
    return len(rows), elapsed_ms, rows[:3]


def normalize_row(rows: list[list]) -> str:
    """Create a comparable string from result rows (sortable)."""
    if not rows:
        return ""
    # Sort by all columns for order-independent comparison
    sorted_rows = sorted(["||".join(str(c).strip() for c in r) for r in rows])
    return "|||".join(sorted_rows)


def rows_match(rows_a: list[list], rows_b: list[list], tolerance=0.01) -> bool:
    """Compare two result sets (order-independent, float tolerance)."""
    if len(rows_a) != len(rows_b):
        return False
    def norm(r):
        return sorted([str(round(float(c), 6)) if _is_float(c) else str(c).strip() for c in r] for r in r)
    na, nb = norm(rows_a), norm(rows_b)
    if na == nb:
        return True
    # Try with 0.1% numeric tolerance
    def norm_tol(r):
        return sorted([_round_float(c, tolerance) for c in r] for r in r)
    return norm_tol(rows_a) == norm_tol(rows_b)


def _is_float(v: str) -> bool:
    try:
        float(str(v).strip())
        return True
    except:
        return False


def _round_float(v: str, tol: float) -> str:
    try:
        f = float(str(v).strip())
        return f"{f:.6f}"
    except:
        return str(v).strip()


# ── Schema ────────────────────────────────────────────────────────────────────

TPCH_SCHEMA_SQLITE = """
CREATE TABLE region (
    r_regionkey INTEGER, r_name TEXT, r_comment TEXT
);
CREATE TABLE nation (
    n_nationkey INTEGER, n_name TEXT, n_regionkey INTEGER, n_comment TEXT
);
CREATE TABLE supplier (
    s_suppkey INTEGER PRIMARY KEY, s_name TEXT, s_address TEXT,
    s_nationkey INTEGER, s_phone TEXT, s_acctbal REAL, s_comment TEXT
);
CREATE TABLE customer (
    c_custkey INTEGER PRIMARY KEY, c_name TEXT, c_address TEXT,
    c_nationkey INTEGER, c_phone TEXT, c_acctbal REAL, c_mktsegment TEXT, c_comment TEXT
);
CREATE TABLE part (
    p_partkey INTEGER PRIMARY KEY, p_name TEXT, p_mfgr TEXT,
    p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT,
    p_retailprice REAL, p_comment TEXT
);
CREATE TABLE partsupp (
    ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER,
    ps_supplycost REAL, ps_comment TEXT,
    PRIMARY KEY (ps_partkey, ps_suppkey)
);
CREATE TABLE orders (
    o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER, o_orderstatus TEXT,
    o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT,
    o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT
);
CREATE TABLE lineitem (
    l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER,
    l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL,
    l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT,
    l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT,
    l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT
);
"""

TPCH_SCHEMA_PG = """
DROP TABLE IF EXISTS lineitem CASCADE;
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS partsupp CASCADE;
DROP TABLE IF EXISTS part CASCADE;
DROP TABLE IF EXISTS customer CASCADE;
DROP TABLE IF EXISTS supplier CASCADE;
DROP TABLE IF EXISTS nation CASCADE;
DROP TABLE IF EXISTS region CASCADE;

CREATE TABLE region (
    r_regionkey INTEGER, r_name TEXT, r_comment TEXT
);
CREATE TABLE nation (
    n_nationkey INTEGER, n_name TEXT, n_regionkey INTEGER, n_comment TEXT
);
CREATE TABLE supplier (
    s_suppkey INTEGER PRIMARY KEY, s_name TEXT, s_address TEXT,
    s_nationkey INTEGER, s_phone TEXT, s_acctbal REAL, s_comment TEXT
);
CREATE TABLE customer (
    c_custkey INTEGER PRIMARY KEY, c_name TEXT, c_address TEXT,
    c_nationkey INTEGER, c_phone TEXT, c_acctbal REAL, c_mktsegment TEXT, c_comment TEXT
);
CREATE TABLE part (
    p_partkey INTEGER PRIMARY KEY, p_name TEXT, p_mfgr TEXT,
    p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT,
    p_retailprice REAL, p_comment TEXT
);
CREATE TABLE partsupp (
    ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER,
    ps_supplycost REAL, ps_comment TEXT,
    PRIMARY KEY (ps_partkey, ps_suppkey)
);
CREATE TABLE orders (
    o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER, o_orderstatus TEXT,
    o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT,
    o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT
);
CREATE TABLE lineitem (
    l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER,
    l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL,
    l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT,
    l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT,
    l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT
);
"""


# ── Server management ─────────────────────────────────────────────────────────

def check_server(host: str, port: int) -> bool:
    rc, _, _ = run_cmd(["nc", "-z", "-w", "2", host, str(port)])
    return rc == 0


def stop_sqlrustgo(port: int):
    """Kill sqlrustgo-mysql-server on given port."""
    run_cmd(["pkill", "-f", f"sqlrustgo-mysql-server.*{port}"])


def start_sqlrustgo(binary: str, data_dir: str, port: int) -> Optional[int]:
    """Start sqlrustgo-mysql-server. Returns PID or None."""
    pid_file = f"/tmp/sqlrustgo-{port}.pid"
    stop_sqlrustgo(port)
    os.makedirs(data_dir, exist_ok=True)
    cmd = [binary, "serve", "--data-dir", data_dir, "--port", str(port)]
    with open(f"/tmp/sqlrustgo-{port}.log", "w") as f:
        p = subprocess.Popen(cmd, stdout=f, stderr=f)
    with open(pid_file, "w") as f:
        f.write(str(p.pid))
    # Wait for ready
    for _ in range(20):
        time.sleep(0.5)
        if check_server("127.0.0.1", port):
            return p.pid
    return None


# ── Main benchmark ─────────────────────────────────────────────────────────────

def benchmark_engine(
    engine: str,
    data_dir: str,
    sf: str,
    schema_sql: str,
    db_url: str,
    mysql_host: str,
    mysql_port: int,
    sqlrustgo_binary: str,
    sqlrustgo_port: int,
    db_name: str,
    verbose: bool = False,
) -> dict:
    """Run full TPC-H 22-query benchmark on one engine."""

    print(f"\n{'='*60}")
    print(f"  {engine.upper()}  (SF={sf})")
    print(f"{'='*60}")

    # ── Load data ─────────────────────────────────────────────────────────────
    load_start = time.perf_counter()
    if engine == "sqlite":
        db_path = f"/tmp/tpch_{engine}_{sf}.db"
        os.makedirs("/tmp", exist_ok=True)
        if Path(db_path).exists():
            os.remove(db_path)
        load_sqlite(db_path, data_dir, schema_sql)
        load_elapsed = (time.perf_counter() - load_start) * 1000

        def run_sql(sql):
            return run_query_sqlite(db_path, sql)

    elif engine == "postgresql":
        load_pg(db_url, data_dir, schema_sql)
        load_elapsed = (time.perf_counter() - load_start) * 1000

        def run_sql(sql):
            return run_query_pg(db_url, sql)

    elif engine == "mysql":
        if not check_server(mysql_host, mysql_port):
            print(f"  [mysql] server not running on {mysql_host}:{mysql_port}")
            return {}
        # Use underscore in DB name (no dots allowed in some envs)
        load_mysql(mysql_host, mysql_port, db_name, data_dir, schema_sql)
        load_elapsed = (time.perf_counter() - load_start) * 1000

        def run_sql(sql):
            return run_query_mysql(mysql_host, mysql_port, db_name, sql)

    elif engine == "sqlrustgo":
        pid = start_sqlrustgo(sqlrustgo_binary, data_dir, sqlrustgo_port)
        if not pid:
            print(f"  [sqlrustgo] failed to start server on port {sqlrustgo_port}")
            return {}
        time.sleep(2)
        load_sqlrustgo("127.0.0.1", sqlrustgo_port, db_name, data_dir)
        load_elapsed = (time.perf_counter() - load_start) * 1000

        def run_sql(sql):
            return run_query_sqlrustgo("127.0.0.1", sqlrustgo_port, db_name, sql)

    print(f"  Data loaded in {load_elapsed:.0f}ms")

    # ── Run queries ───────────────────────────────────────────────────────────
    results = {}
    total_ms = 0.0
    for q in range(1, 23):
        sql = read_query(q)
        if not sql:
            print(f"  Q{q:02d}: SKIP (query file not found)")
            continue

        rows, dur_ms, sample = run_sql(sql)
        total_ms += dur_ms
        status = "OK" if rows >= 0 else "ERR"
        print(f"  Q{q:02d}: {status} | {rows} rows | {dur_ms:8.1f}ms")
        if verbose and sample:
            print(f"         sample: {sample[0][:3]}")
        results[f"Q{q:02d}"] = {
            "rows": rows,
            "duration_ms": round(dur_ms, 2),
            "sample": sample[:3] if sample else [],
            "signature": normalize_row(sample[:3]) if sample else "",
        }

    print(f"  TOTAL: {total_ms:.0f}ms across {len(results)} queries")

    # Cleanup
    if engine == "sqlrustgo":
        stop_sqlrustgo(sqlrustgo_port)

    return results


def build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="TPC-H Multi-Engine Benchmark")
    p.add_argument("--sf", "--scale", dest="sf", default="sf0.01",
                   choices=["sf0.001", "sf0.01", "sf1"], help="Scale factor")
    p.add_argument("--engines", default="sqlite,postgresql,mysql,sqlrustgo",
                   help="Comma-separated engines to test")
    p.add_argument("--data-root", default="/tmp/tpch_benchmark",
                   help="Root directory containing sf*_data/ subdirectories")
    p.add_argument("--queries-dir", default=None,
                   help="Override queries/ directory path")
    p.add_argument("--baseline", default=None,
                   help="Path to baseline JSON to compare against")
    p.add_argument("--capture", action="store_true",
                   help="Capture baseline instead of comparing")
    p.add_argument("--output", default=None,
                   help="Output JSON path (default: /tmp/tpch_benchmark/results_{sf}_{engine}.json)")
    p.add_argument("--sqlrustgo-binary", default=None,
                   help="Path to sqlrustgo-mysql-server binary")
    p.add_argument("--pg-url", default="postgresql://liying@/tpch_sf01?host=/tmp",
                   help="PostgreSQL connection URL")
    p.add_argument("--mysql-host", default="127.0.0.1",
                   help="MySQL host")
    p.add_argument("--mysql-port", type=int, default=3306,
                   help="MySQL port (default 3306 for MariaDB)")
    p.add_argument("--sqlrustgo-port", type=int, default=3308,
                   help="SQLRustGo wire port")
    p.add_argument("--verbose", action="store_true")
    return p


def main():
    args = build_arg_parser().parse_args()

    # Sanitize DB name (replace dots with underscores for cross-engine compat)
    sf_db_safe = args.sf.replace(".", "_")
    db_name = f"tpch_{sf_db_safe}"

    global TPCH_QUERIES_DIR
    if args.queries_dir:
        TPCH_QUERIES_DIR = Path(args.queries_dir)

    engines = [e.strip() for e in args.engines.split(",")]
    data_dir = f"{args.data_root}/{args.sf}_data"

    if not Path(data_dir).exists():
        print(f"ERROR: Data directory not found: {data_dir}")
        print(f"Available: {[p.name for p in Path(args.data_root).iterdir() if p.is_dir()]}")
        sys.exit(1)

    binary = args.sqlrustgo_binary or "/Users/liying/workspace/dev/yinglichina163/sqlrustgo/target/debug/sqlrustgo-mysql-server"

    # ── Capture baseline ────────────────────────────────────────────────────────
    if args.capture:
        baseline = {}
        # In --capture mode, only run explicitly requested engines (avoid slow sqlrustgo startup)
        capture_engines = engines if engines else ["sqlite", "postgresql", "mysql"]
        for engine in capture_engines:
            r = benchmark_engine(
                engine=engine,
                data_dir=data_dir,
                sf=args.sf,
                schema_sql=TPCH_SCHEMA_SQLITE,
                db_url=args.pg_url,
                mysql_host=args.mysql_host,
                mysql_port=args.mysql_port,
                sqlrustgo_binary=binary,
                sqlrustgo_port=args.sqlrustgo_port,
                db_name=db_name,
                verbose=args.verbose,
            )
            baseline[engine] = r

        out_path = args.output or f"/tmp/tpch_benchmark/baseline_{args.sf}.json"
        Path(out_path).parent.mkdir(parents=True, exist_ok=True)
        Path(out_path).write_text(json.dumps(baseline, indent=2))
        print(f"\nBaseline captured → {out_path}")
        return

    # ── Compare against baseline ───────────────────────────────────────────────
    baseline_path = args.baseline or f"/tmp/tpch_benchmark/baseline_{args.sf}.json"
    if not Path(baseline_path).exists():
        print(f"ERROR: Baseline not found: {baseline_path}")
        print("Run with --capture first.")
        sys.exit(1)

    baseline = json.loads(Path(baseline_path).read_text())
    all_results = {}
    summary = []

    for engine in engines:
        results = benchmark_engine(
            engine=engine,
            data_dir=data_dir,
            sf=args.sf,
            schema_sql=TPCH_SCHEMA_SQLITE,
            db_url=args.pg_url,
            mysql_host=args.mysql_host,
            mysql_port=args.mysql_port,
            sqlrustgo_binary=binary,
            sqlrustgo_port=args.sqlrustgo_port,
            db_name=db_name,
            verbose=args.verbose,
        )
        all_results[engine] = results

        if engine not in baseline:
            print(f"  [WARN] {engine} not in baseline — capturing as new")
            baseline[engine] = results

    # ── Comparison report ──────────────────────────────────────────────────────
    print(f"\n{'='*70}")
    print(f"  COMPARISON REPORT  (SF={args.sf})")
    print(f"{'='*70}")

    # Use SQLite as reference if available, else first engine
    ref = baseline.get("sqlite", {})
    engines_in_results = list(all_results.keys())

    for q in range(1, 23):
        qname = f"Q{q:02d}"
        ref_data = ref.get(qname, {})
        ref_rows = ref_data.get("rows", -1)
        ref_sig = ref_data.get("signature", "")

        row = {"query": qname, "reference": ref_rows}
        match = True
        for eng in engines_in_results:
            eng_data = all_results[eng].get(qname, {})
            eng_rows = eng_data.get("rows", -1)
            eng_sig = eng_data.get("signature", "")
            status = "OK" if eng_rows == ref_rows else "MISMATCH"
            if eng_rows != ref_rows:
                match = False
            row[eng] = eng_rows
            row[f"{eng}_ms"] = eng_data.get("duration_ms", 0)

        row["match"] = match
        summary.append(row)

        ref_row_str = f"{ref_rows:>6}"
        eng_strs = []
        for eng in engines_in_results:
            v = row.get(eng, "N/A")
            m = "✓" if v == ref_rows else "✗"
            eng_strs.append(f"{m}{eng[:4]}={v}")
        print(f"  {qname} ref={ref_row_str}  {'  '.join(eng_strs)}")

    # ── Summary table ────────────────────────────────────────────────────────
    print(f"\n{'='*70}")
    print(f"  PASS/FAIL SUMMARY")
    print(f"{'='*70}")
    header = f"  {'Query':<6}" + "".join(f"  {e[:4]:>8}" for e in engines_in_results) + f"  {'Match':>6}"
    print(header)
    print("  " + "-"*60)
    pass_count = {"_total": 0}
    for eng in engines_in_results:
        pass_count[eng] = 0

    for row in summary:
        q = row["query"]
        match_all = all(row.get(eng, -1) == row["reference"] for eng in engines_in_results)
        if match_all:
            pass_count["_total"] += 1
            for eng in engines_in_results:
                if row.get(eng, -1) == row["reference"]:
                    pass_count[eng] = pass_count.get(eng, 0) + 1

        eng_vals = "".join(f"  {row.get(e, 'N/A'):>8}" for e in engines_in_results)
        match_sym = "✓ PASS" if row["match"] else "✗ FAIL"
        print(f"  {q:<6}{eng_vals}  {match_sym}")

    total = len(summary)
    print(f"\n  PASS: {pass_count['_total']}/{total} queries matched reference")
    for eng in engines_in_results:
        pct = pass_count.get(eng, 0) / max(total, 1) * 100
        print(f"    {eng}: {pass_count.get(eng, 0)}/{total} ({pct:.0f}%)")

    # ── Save results ───────────────────────────────────────────────────────────
    out_path = args.output or f"/tmp/tpch_benchmark/results_{args.sf}.json"
    Path(out_path).parent.mkdir(parents=True, exist_ok=True)
    output = {
        "sf": args.sf,
        "engines": engines_in_results,
        "reference": "sqlite",
        "summary": summary,
        "all_results": all_results,
        "baseline": baseline,
    }
    Path(out_path).write_text(json.dumps(output, indent=2))
    print(f"\nResults → {out_path}")


if __name__ == "__main__":
    main()
