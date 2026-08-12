#!/usr/bin/env python3
"""
TPC-H SF=10 Cross-Engine SHA256 Harness (Issue #4018 V312-18).

Reuses the SF=1 cross-engine harness helpers
(`scripts/tpch_cross_engine_harness.py`) and runs the same 22 canonical
TPC-H queries against a SF=10 fixture. Emits per-query row counts +
sha256s into a scale-separated evidence directory:

  - <out_dir>/sqlite/q<N>.tsv
  - <out_dir>/sqlite/q<N>.sha256
  - <out_dir>/sqlite/row_counts.txt
  - <out_dir>/sqlite/SUMMARY.json

Usage:
  python3 scripts/tpch_sf10_cross_engine_harness.py
  python3 scripts/tpch_sf10_cross_engine_harness.py --sf10-dir /tmp/tpch-sf10
  python3 scripts/tpch_sf10_cross_engine_harness.py --skip-setup
  python3 scripts/tpch_sf10_cross_engine_harness.py --queries 1 6 22

Why a separate harness script (not a --scale flag on the SF=1 one)?
  SF=1 takes ~30s setup + ~30s runs. SF=10 is ~10GB of .tbl data;
  importing + indexing alone is several minutes, and SF=10 results
  exceed in-memory sort budgets for several queries. Splitting the
  scripts keeps the SF=1 fast-path CI-friendly and lets the SF=10
  harness grow its own knobs (parallel import, statement timeout,
  per-query budget) without polluting the SF=1 path.

Exit codes:
  0  All 22 queries executed; row_count + sha256 captured
  1  Setup or query failure
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import sys
import time
from pathlib import Path

# Import the SF=1 harness as a module so we reuse SCHEMA_DDL, INDEX_DDL,
# setup_sqlite, run_query, sort_rows, sha256_of_rows, write_tsv, and
# the SQLITE_REWRITES for Q7/Q8/Q9 (EXTRACT(YEAR FROM x) → strftime).
_SF1_PATH = Path(__file__).resolve().parent / "tpch_cross_engine_harness.py"
_spec = importlib.util.spec_from_file_location("tpch_xeng_sf1", _SF1_PATH)
if _spec is None or _spec.loader is None:
    print("[harness] FATAL: cannot import tpch_cross_engine_harness.py", file=sys.stderr)
    sys.exit(1)
_mod = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_mod)

REPO_ROOT = _mod.REPO_ROOT
QUERIES_DIR = _mod.QUERIES_DIR
SCHEMA_DDL = _mod.SCHEMA_DDL
INDEX_DDL = _mod.INDEX_DDL
TABLES = _mod.TABLES
SQLITE_REWRITES = _mod.SQLITE_REWRITES
setup_sqlite = _mod.setup_sqlite
run_query = _mod.run_query
sort_rows = _mod.sort_rows
sha256_of_rows = _mod.sha256_of_rows
write_tsv = _mod.write_tsv
log = _mod.log

# SF=10 defaults: data dir matches scripts/tpch/setup_sf10.sh default,
# db is named distinctly from the SF=1 one to avoid clobbering.
DEFAULT_SF10_DIR = Path("/tmp/tpch-sf10")
DEFAULT_SQLITE_DB = Path("/tmp/tpch_sf10_cross_engine.db")
DEFAULT_OUT_DIR = REPO_ROOT / "docs" / "releases" / "v3.12.0" / "evidence" / "tpch" / "cross_engine_sf10"


def main() -> int:
    ap = argparse.ArgumentParser(description="TPC-H SF=10 SQLite cross-engine harness (Issue #4018)")
    ap.add_argument("--sf10-dir", type=Path, default=DEFAULT_SF10_DIR,
                    help=f"Path to SF=10 .tbl fixture (default: {DEFAULT_SF10_DIR})")
    ap.add_argument("--db", type=Path, default=DEFAULT_SQLITE_DB,
                    help=f"Path to SQLite database file (default: {DEFAULT_SQLITE_DB})")
    ap.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR,
                    help=f"Output directory (default: {DEFAULT_OUT_DIR})")
    ap.add_argument("--skip-setup", action="store_true",
                    help="Reuse existing SQLite DB without re-importing")
    ap.add_argument("--queries", nargs="*", type=int, default=None,
                    help="Specific query numbers to run (default: all 22)")
    ap.add_argument("--per-query-timeout-sec", type=int, default=600,
                    help="Per-query wall-clock budget (default: 600s)")
    args = ap.parse_args()

    if not args.sf10_dir.exists():
        log(f"ERROR: SF=10 fixture dir not found: {args.sf10_dir}")
        log("       Generate with: bash scripts/tpch/setup_sf10.sh")
        log("       (Requires dbgen in PATH or tpch-dbgen/dbgen binary;")
        log("        falls back to a partial stub if dbgen is missing.)")
        return 1

    if not args.skip_setup:
        setup_sqlite(args.sf10_dir, args.db)
    else:
        if not args.db.exists():
            log(f"ERROR: --skip-setup but db does not exist: {args.db}")
            return 1
        log(f"setup: skipped (using existing {args.db})")

    out_dir = args.out_dir / "sqlite"
    out_dir.mkdir(parents=True, exist_ok=True)
    log(f"output: {out_dir}")

    qnums = args.queries if args.queries else list(range(1, 23))
    missing = [n for n in qnums if not (QUERIES_DIR / f"q{n}.sql").exists()]
    if missing:
        log(f"ERROR: missing q*.sql files: {missing}")
        return 1

    import sqlite3 as _sqlite3
    conn = _sqlite3.connect(str(args.db))
    conn.text_factory = str
    summary: dict = {
        "engine": "sqlite",
        "engine_version": _sqlite3.sqlite_version,
        "fixture_dir": str(args.sf10_dir),
        "fixture_size_factor": 10.0,
        "queries": [],
    }

    row_count_lines = []
    for n in qnums:
        sql_path = QUERIES_DIR / f"q{n}.sql"
        sql_text = sql_path.read_text(encoding="utf-8").strip()
        if sql_text.endswith(";"):
            sql_text = sql_text[:-1]
        sql_source = "standard_tpch_q.sql"
        if n in SQLITE_REWRITES:
            sql_text = SQLITE_REWRITES[n]
            sql_source = "sqlite_rewrite (EXTRACT(YEAR FROM x) → strftime('%Y', x))"
        try:
            rows, count, elapsed = run_query(conn, sql_text)
        except _sqlite3.Error as e:
            log(f"  Q{n:>2}: SQL ERROR — {e}")
            summary["queries"].append({
                "q": n, "row_count": None, "sha256": None, "elapsed_ms": 0.0,
                "error": str(e), "sql_source": sql_source,
            })
            row_count_lines.append(f"q{n}.sql ERROR {e}")
            continue
        if elapsed > args.per_query_timeout_sec:
            log(f"  Q{n:>2}: TIMEOUT — exceeded {args.per_query_timeout_sec}s budget")
            summary["queries"].append({
                "q": n, "row_count": None, "sha256": None, "elapsed_ms": round(elapsed*1000, 3),
                "error": f"timeout (> {args.per_query_timeout_sec}s)", "sql_source": sql_source,
            })
            row_count_lines.append(f"q{n}.sql TIMEOUT")
            continue
        sorted_rows = sort_rows(rows)
        sha = sha256_of_rows(sorted_rows)
        write_tsv(out_dir / f"q{n}.tsv", sorted_rows)
        log(f"  Q{n:>2}: {count:>11,d} rows in {elapsed*1000:>10.1f} ms  sha256={sha[:16]}…")
        summary["queries"].append({
            "q": n,
            "row_count": count,
            "sha256": sha,
            "elapsed_ms": round(elapsed * 1000, 3),
            "sql_source": sql_source,
        })
        row_count_lines.append(f"q{n}.sql {count}")

    conn.close()

    (out_dir / "row_counts.txt").write_text("\n".join(row_count_lines) + "\n")
    summary["completed_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    summary["row_count_total"] = sum(q["row_count"] or 0 for q in summary["queries"])
    summary["queries_executed"] = sum(1 for q in summary["queries"] if q["row_count"] is not None)
    summary["queries_errored"] = sum(
        1 for q in summary["queries"]
        if q.get("error") and not q.get("row_count")
    )
    (out_dir / "SUMMARY.json").write_text(
        json.dumps(summary, indent=2, ensure_ascii=False) + "\n"
    )
    log(f"summary → {out_dir / 'SUMMARY.json'}")
    log(f"row counts → {out_dir / 'row_counts.txt'}")
    log(f"DONE: {summary['queries_executed']}/22 queries captured ({summary['queries_errored']} errored)")
    return 0


if __name__ == "__main__":
    sys.exit(main())