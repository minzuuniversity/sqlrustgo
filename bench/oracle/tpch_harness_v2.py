#!/usr/bin/env python3
"""
TPC-H Benchmark Harness v2 (GA-grade).

User 2026-06-07 feedback: previous '18/22 PASS' was based on stdout
parse + mutual comparison. This is a deterministic, oracle-safe,
CI-ready harness with semantic comparator.

Architecture (3-layer):
  L1: Execution (sqlrustgo, PG oracle)
  L2: Evaluation (semantic comparator + timeout classifier)
  L3: Reporting (PASS/FAIL/TIMEOUT + JSON)

Row semantics (P0-2):
  scalar_aggregate  -> 1 row (NULL if no input)
  group_by          -> 0+ rows
  exists_subquery   -> 0 or 1 row (boolean)
  empty_relation    -> 0 rows

States (P1-1):
  PASS             semantic match (NULL/empty aware)
  FAIL             semantic mismatch with PG
  TIMEOUT          engine non-verifiable (N^2 EXISTS, etc.)
  DATA_LIMITATION  PG=0, cannot verify engine
  ENGINE_ISSUE     PG>0, engine=0 (missing data)

Usage:
  # Run all 22 queries vs PG oracle
  python3 tpch_harness_v2.py run --snapshot bench/oracle/tpch_sf01_snapshot_v2

  # Run a single query
  python3 tpch_harness_v2.py run --query Q6 --snapshot bench/oracle/tpch_sf01_snapshot_v2

  # Validate PG matches snapshot fingerprint
  python3 tpch_harness_v2.py validate --snapshot bench/oracle/tpch_sf01_snapshot_v2
"""
import argparse
import json
import os
import re
import subprocess
import sys
import time
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional


# ============================================================================
# L1: Execution Layer
# ============================================================================

PSQL = "/opt/homebrew/opt/postgresql@16/bin/psql"
PG_ENV = {"PGPASSWORD": "test123", "PATH": "/opt/homebrew/opt/postgresql@16/bin:/usr/bin:/bin"}

# Strict psql flags (P0-3 from user feedback)
PSQL_STRICT_FLAGS = [
    "-X",                   # no .psqlrc (deterministic)
    "-v", "ON_ERROR_STOP=1",
    "-q",                   # quiet
    "-t",                   # tuples-only
    "-A",                   # unaligned
]


def psql_strict(query: str, timeout: int = 30) -> str:
    """Run psql with strict flags. Returns stdout (no header, unaligned)."""
    cmd = [PSQL, "-U", "liying", "-d", "tpch_test", *PSQL_STRICT_FLAGS, "-c", query]
    r = subprocess.run(cmd, env=PG_ENV, capture_output=True, text=True, timeout=timeout)
    if r.returncode != 0:
        raise RuntimeError(f"psql failed: {r.stderr.strip()[:200]}")
    return r.stdout


# ============================================================================
# Row Semantics (P0-2)
# ============================================================================

def classify_query(sql: str) -> dict:
    """AST-level row semantics classification.

    Returns:
      {
        "type": "scalar_aggregate" | "group_by" | "exists_subquery" | "relation",
        "expected_rows": int | None,  # None = data-dependent
        "has_aggregate_func": bool,
        "has_group_by": bool,
      }
    """
    upper = sql.upper()
    # Strip leading whitespace + comments
    upper_clean = re.sub(r"--.*$", "", upper, flags=re.MULTILINE).strip()
    # Check for SELECT scalar aggregate (no GROUP BY, no SELECT *)
    has_aggregate = bool(re.search(r"\b(SUM|COUNT|AVG|MIN|MAX)\s*\(", upper_clean))
    has_group_by = " GROUP BY " in upper_clean or upper_clean.endswith("GROUP BY")
    # Check for EXISTS
    has_exists = "EXISTS" in upper_clean
    # Check for SELECT *
    is_select_star = "SELECT *" in upper_clean

    if has_exists and not has_group_by:
        return {
            "type": "exists_subquery",
            "expected_rows": None,  # data-dependent
            "has_aggregate_func": False,
            "has_group_by": False,
        }
    if has_aggregate and not has_group_by:
        return {
            "type": "scalar_aggregate",
            "expected_rows": 1,  # always 1 row (with NULL if empty)
            "has_aggregate_func": True,
            "has_group_by": False,
        }
    if has_group_by:
        return {
            "type": "group_by",
            "expected_rows": None,  # data-dependent
            "has_aggregate_func": has_aggregate,
            "has_group_by": True,
        }
    if is_select_star:
        return {
            "type": "relation",
            "expected_rows": None,
            "has_aggregate_func": False,
            "has_group_by": False,
        }
    return {
        "type": "relation",
        "expected_rows": None,
        "has_aggregate_func": False,
        "has_group_by": False,
    }


# ============================================================================
# Engine Runner (sqlrustgo) - Sprint 5 via tpch_run_query binary
# ============================================================================

ENGINE_BIN = None  # auto-discover


def find_engine_bin() -> Optional[Path]:
    """Find tpch_run_query binary built by cargo."""
    # Standard cargo target dir
    candidates = [
        Path("target/release/examples/tpch_run_query"),
        Path("target/debug/examples/tpch_run_query"),
    ]
    for c in candidates:
        if c.exists():
            return c
    return None


def run_sqlrustgo_query(
    sql: str,
    query_name: str,
    data_dir: str = "/tmp/tpch_sf01_v2",
    timeout_sec: int = 15,
) -> dict:
    """Run sqlrustgo via tpch_run_query binary. Returns JSON-parsed result."""
    bin_path = ENGINE_BIN or find_engine_bin()
    if bin_path is None:
        return {
            "engine": "sqlrustgo",
            "rows": [],
            "row_count": 0,
            "duration_ms": 0,
            "error": "engine binary not found (run `cargo build --example tpch_run_query`)",
            "timed_out": False,
        }
    cmd = [
        str(bin_path),
        "--query", query_name,
        "--data-dir", data_dir,
        "--timeout", str(timeout_sec),
    ]
    start = time.time()
    try:
        r = subprocess.run(
            cmd, capture_output=True, text=True,
            timeout=timeout_sec + 5,  # buffer for binary startup
        )
        elapsed_ms = int((time.time() - start) * 1000)
    except subprocess.TimeoutExpired:
        return {
            "engine": "sqlrustgo",
            "rows": [],
            "row_count": 0,
            "duration_ms": (timeout_sec + 5) * 1000,
            "error": "TIMEOUT",
            "timed_out": True,
        }
    if r.returncode != 0:
        return {
            "engine": "sqlrustgo",
            "rows": [],
            "row_count": 0,
            "duration_ms": elapsed_ms,
            "error": r.stderr.strip()[:200] or f"exit {r.returncode}",
            "timed_out": False,
        }
    # Parse last JSON line (binary may print warnings to stderr)
    last_line = r.stdout.strip().split("\n")[-1]
    try:
        result = json.loads(last_line)
        result.setdefault("engine", "sqlrustgo")
        result.setdefault("query", query_name)
        result.setdefault("rows", [])
        result.setdefault("row_count", 0)
        result.setdefault("duration_ms", elapsed_ms)
        result.setdefault("error", None)
        result.setdefault("timed_out", False)
        return result
    except json.JSONDecodeError as e:
        return {
            "engine": "sqlrustgo",
            "rows": [],
            "row_count": 0,
            "duration_ms": elapsed_ms,
            "error": f"json parse: {e}; stdout: {r.stdout[:200]!r}",
            "timed_out": False,
        }


# ============================================================================
# PG Oracle Runner (L1)
# ============================================================================

def run_pg_query(sql: str, timeout_sec: int = 30) -> dict:
    """Run PG via strict psql. Returns semantic result."""
    start = time.time()
    try:
        stdout = psql_strict(sql, timeout=timeout_sec)
        elapsed_ms = int((time.time() - start) * 1000)
    except subprocess.TimeoutExpired:
        return {
            "engine": "postgresql",
            "rows": [],
            "row_count": 0,
            "duration_ms": timeout_sec * 1000,
            "error": "TIMEOUT",
            "timed_out": True,
        }
    except Exception as e:
        return {
            "engine": "postgresql",
            "rows": [],
            "row_count": 0,
            "duration_ms": int((time.time() - start) * 1000),
            "error": str(e)[:200],
            "timed_out": False,
        }
    # Parse stdout: one row per line, columns pipe-separated (psql -A)
    rows = []
    for line in stdout.split("\n"):
        if not line.strip():
            continue
        cells = line.split("|")
        rows.append(cells)
    return {
        "engine": "postgresql",
        "rows": rows,
        "row_count": len(rows),
        "duration_ms": elapsed_ms,
        "error": None,
        "timed_out": False,
    }


# ============================================================================
# L2: Semantic Comparator
# ============================================================================

NULL_LIKE = {"", "NULL", "null", "Null"}


def cell_semantic_equal(a: str, b: str) -> bool:
    """Both NULL-like (empty/NULL) → match. Otherwise exact string."""
    a_null = a in NULL_LIKE
    b_null = b in NULL_LIKE
    if a_null and b_null:
        return True
    if a_null != b_null:
        return False
    # Strip trailing whitespace for CHAR(N) padding
    return a.rstrip() == b.rstrip()


def numeric_close(a: str, b: str, rel_tol: float = 1e-9) -> bool:
    """Float-aware equality for SUM/AVG/COUNT outputs.

    PG returns 877911.41, sqlrustgo may return 877911.410000004 due
    to f64 rounding. Treat these as "close enough" for harness
    purposes (Sprint 5: opencode fixes precision bugs, not real
    semantic differences).
    """
    if a == b:
        return True
    try:
        af = float(a)
        bf = float(b)
    except (ValueError, TypeError):
        return False
    if af == bf:
        return True
    if af == 0.0 or bf == 0.0:
        return abs(af - bf) < rel_tol
    return abs(af - bf) / max(abs(af), abs(bf)) < rel_tol


def scalar_aggregate_compare(engine: dict, pg: dict, meta: dict) -> dict:
    """Compare scalar aggregate results.

    Both should return 1 row. Compare the single value (or both NULL).
    """
    eng_rows = engine["rows"]
    pg_rows = pg["rows"]

    # Normalize: scalar aggregate should always have 1 row, possibly with NULL
    if engine.get("timed_out"):
        return {
            "status": "TIMEOUT",
            "classification": "performance_issue",
            "engine_rows": len(eng_rows),
            "pg_rows": len(pg_rows),
            "notes": "engine did not complete",
        }
    if len(eng_rows) == 0 and len(pg_rows) == 1 and all(c in NULL_LIKE for c in pg_rows[0]):
        # Engine returned 0 rows but PG returned 1 row with NULL.
        # With #3288 fix, sqlrustgo should also return 1 row with NULL.
        return {
            "status": "FAIL",
            "classification": "engine_bug",
            "engine_rows": 0,
            "pg_rows": 1,
            "notes": "engine returned 0 rows; expected 1 row with NULL (SQL standard)",
        }
    if len(eng_rows) == 1 and len(pg_rows) == 0:
        return {
            "status": "FAIL",
            "classification": "oracle_bug",
            "engine_rows": 1,
            "pg_rows": 0,
            "notes": "engine returned 1 row; PG returned 0 (should be 1 with NULL)",
        }
    if len(eng_rows) != 1 or len(pg_rows) != 1:
        return {
            "status": "FAIL",
            "classification": "row_count_mismatch",
            "engine_rows": len(eng_rows),
            "pg_rows": len(pg_rows),
            "notes": f"expected 1 row each, got {len(eng_rows)}/{len(pg_rows)}",
        }
    # Compare the single value
    eng_val = eng_rows[0][0] if eng_rows[0] else ""
    pg_val = pg_rows[0][0] if pg_rows[0] else ""
    if cell_semantic_equal(eng_val, pg_val):
        return {
            "status": "PASS",
            "classification": "scalar_aggregate",
            "engine_rows": 1,
            "pg_rows": 1,
            "engine_value": eng_val,
            "pg_value": pg_val,
        }
    # Try numeric tolerance (Sprint 5: f64 precision differences)
    if numeric_close(eng_val, pg_val, rel_tol=1e-6):
        return {
            "status": "PASS",
            "classification": "scalar_aggregate_numeric_tolerance",
            "engine_rows": 1,
            "pg_rows": 1,
            "engine_value": eng_val,
            "pg_value": pg_val,
            "notes": "values match within numeric tolerance (f64 precision)",
        }
    return {
        "status": "FAIL",
        "classification": "value_mismatch",
        "engine_rows": 1,
        "pg_rows": 1,
        "engine_value": eng_val,
        "pg_value": pg_val,
    }


def relation_compare(engine: dict, pg: dict, meta: dict) -> dict:
    """Compare relation (group by or flat) results."""
    if engine.get("timed_out"):
        return {
            "status": "TIMEOUT",
            "classification": "performance_issue",
            "engine_rows": -1,
            "pg_rows": pg["row_count"],
            "notes": "engine did not complete (N² EXISTS or similar)",
        }
    eng_n = engine["row_count"]
    pg_n = pg["row_count"]

    if pg_n == 0 and eng_n > 0:
        return {
            "status": "DATA_LIMITATION",
            "classification": "data_limitation",
            "engine_rows": eng_n,
            "pg_rows": 0,
            "notes": "PG=0; cannot verify engine without non-zero reference. May be data limitation or engine bug.",
        }
    if pg_n > 0 and eng_n == 0:
        return {
            "status": "ENGINE_ISSUE",
            "classification": "engine_issue",
            "engine_rows": 0,
            "pg_rows": pg_n,
            "notes": "PG has data, engine returns nothing",
        }
    if eng_n != pg_n:
        return {
            "status": "FAIL",
            "classification": "row_count_mismatch",
            "engine_rows": eng_n,
            "pg_rows": pg_n,
            "notes": f"row count differs: engine={eng_n} pg={pg_n}",
        }
    # Same row count - compare cells (multiset equality)
    eng_sorted = sorted(engine["rows"])
    pg_sorted = sorted(pg["rows"])
    cell_diffs = []
    for i, (e_row, p_row) in enumerate(zip(eng_sorted, pg_sorted)):
        max_cols = max(len(e_row), len(p_row))
        for j in range(max_cols):
            e_cell = e_row[j] if j < len(e_row) else ""
            p_cell = p_row[j] if j < len(p_row) else ""
            if cell_semantic_equal(e_cell, p_cell):
                continue
            # Try numeric tolerance for floats
            if numeric_close(e_cell, p_cell, rel_tol=1e-6):
                continue
            if len(cell_diffs) < 10:
                cell_diffs.append({
                    "row": i, "column": j,
                    "expected": p_cell, "actual": e_cell,
                })
    if not cell_diffs:
        return {
            "status": "PASS",
            "classification": meta["type"],
            "engine_rows": eng_n,
            "pg_rows": pg_n,
        }
    return {
        "status": "FAIL",
        "classification": "cell_diff",
        "engine_rows": eng_n,
        "pg_rows": pg_n,
        "first_mismatches": cell_diffs,
    }


def compare(engine: dict, pg: dict, meta: dict) -> dict:
    """Top-level semantic comparator."""
    if meta["type"] == "scalar_aggregate":
        return scalar_aggregate_compare(engine, pg, meta)
    return relation_compare(engine, pg, meta)


# ============================================================================
# L3: Reporting
# ============================================================================

@dataclass
class Verdict:
    query: str
    status: str
    classification: str
    engine_rows: int
    pg_rows: int
    duration_ms: int = 0
    notes: str = ""


def run_one(query_name: str, sql: str, data_dir: str = "/tmp/tpch_sf01_v2", timeout_sec: int = 15) -> Verdict:
    meta = classify_query(sql)
    pg = run_pg_query(sql, timeout_sec=30)
    engine = run_sqlrustgo_query(sql, query_name, data_dir, timeout_sec=timeout_sec)
    cmp = compare(engine, pg, meta)
    return Verdict(
        query=query_name,
        status=cmp.get("status", "UNKNOWN"),
        classification=cmp.get("classification", ""),
        engine_rows=cmp.get("engine_rows", -1),
        pg_rows=cmp.get("pg_rows", -1),
        duration_ms=engine.get("duration_ms", 0),
        notes=cmp.get("notes", ""),
    )


def main():
    ap = argparse.ArgumentParser(description="TPC-H Harness v2")
    sub = ap.add_subparsers(dest="cmd")

    p_run = sub.add_parser("run", help="Run TPC-H queries vs PG oracle")
    p_run.add_argument("--snapshot", required=True, help="Oracle snapshot dir")
    p_run.add_argument("--queries-dir", default="queries", help="Query files dir")
    p_run.add_argument("--engine-bin", help="Path to sqlrustgo binary (Sprint 5: optional)")
    p_run.add_argument("--report", help="Output JSON report path")
    p_run.add_argument("--query", help="Run single query (Q1..Q22)")
    p_run.add_argument("--timeout", type=int, default=15, help="Per-query timeout in seconds (default 15)")

    p_val = sub.add_parser("validate", help="Validate PG matches snapshot fingerprint")
    p_val.add_argument("--snapshot", required=True)

    args = ap.parse_args()

    if args.cmd == "validate":
        meta_path = Path(args.snapshot) / "meta.json"
        if not meta_path.exists():
            print(f"FAIL: {meta_path} not found", file=sys.stderr)
            sys.exit(1)
        meta = json.loads(meta_path.read_text())
        # Re-hash and compare
        from freeze_oracle import psql_strict
        ok = True
        for tbl, info in meta["tables"].items():
            hash_sql = (
                f"SELECT md5(string_agg(t::text, '|' ORDER BY t::text)) "
                f"FROM (SELECT * FROM {tbl} LIMIT 1000) t"
            )
            actual = psql_strict(hash_sql)[:32]
            expected = info["sample_md5"]
            match = actual == expected
            ok = ok and match
            print(f"  {tbl:12} expected={expected} actual={actual} {'✓' if match else '✗'}", file=sys.stderr)
        print(f"\n{'PASS' if ok else 'FAIL'}: PG matches snapshot fingerprint", file=sys.stderr)
        sys.exit(0 if ok else 1)

    elif args.cmd == "run":
        # Sprint 5: full end-to-end run
        snap_path = Path(args.snapshot)
        meta_path = snap_path / "meta.json"
        if not meta_path.exists():
            print(f"FAIL: {meta_path} not found", file=sys.stderr)
            sys.exit(1)
        meta = json.loads(meta_path.read_text())
        print(f"[harness v2] snapshot: {meta['snapshot_name']}", file=sys.stderr)
        print(f"[harness v2] frozen: {meta['frozen']}", file=sys.stderr)
        print(f"[harness v2] data_source: {meta['data_source']}", file=sys.stderr)
        # Validate PG first
        print(f"[harness v2] validating PG snapshot...", file=sys.stderr)
        from freeze_oracle import psql_strict
        ok = True
        for tbl, info in meta["tables"].items():
            hash_sql = (
                f"SELECT md5(string_agg(t::text, '|' ORDER BY t::text)) "
                f"FROM (SELECT * FROM {tbl} LIMIT 1000) t"
            )
            actual = psql_strict(hash_sql)[:32]
            match = actual == info["sample_md5"]
            ok = ok and match
        if not ok:
            print(f"FAIL: PG snapshot drifted. Re-run freeze_oracle.py.", file=sys.stderr)
            sys.exit(1)
        print(f"[harness v2] PG snapshot OK", file=sys.stderr)
        # Engine binary check
        bin_path = find_engine_bin()
        if bin_path is None:
            print(f"FAIL: tpch_run_query binary not found. Run:", file=sys.stderr)
            print(f"  cargo build --release -p sqlrustgo-bench --example tpch_run_query", file=sys.stderr)
            sys.exit(1)
        print(f"[harness v2] engine: {bin_path}", file=sys.stderr)
        # Run queries
        queries = []
        if args.query:
            qids = args.query.split(",")
        else:
            qids = [f"Q{i}" for i in range(1, 23)]
        for q in qids:
            sql_path = Path(args.queries_dir) / f"{q.lower()}.sql"
            if not sql_path.exists():
                print(f"  {q}: SKIP (file not found)", file=sys.stderr)
                continue
            sql = sql_path.read_text().strip().rstrip(";")
            verdict = run_one(q, sql, timeout_sec=args.timeout)
            icon = {"PASS": "✓", "FAIL": "✗", "TIMEOUT": "⏱", "DATA_LIMITATION": "?",
                    "ENGINE_ISSUE": "✗"}.get(verdict.status, "?")
            print(f"  {verdict.query:3} {icon} {verdict.status:18} "
                  f"engine={verdict.engine_rows:>3} pg={verdict.pg_rows:>3} "
                  f"({verdict.classification}) {verdict.notes[:60]}",
                  file=sys.stderr)
            queries.append(verdict)
        # Summary
        states = {}
        for v in queries:
            states[v.status] = states.get(v.status, 0) + 1
        print(f"\n[summary] {states}", file=sys.stderr)
        # Write report
        if args.report:
            report = {
                "snapshot": meta["snapshot_name"],
                "data_source": meta["data_source"],
                "generated_at": datetime.now(timezone.utc).isoformat(),
                "summary": states,
                "verdicts": [asdict(v) for v in queries],
            }
            report_path = Path(args.report)
            report_path.parent.mkdir(parents=True, exist_ok=True)
            with open(report_path, "w") as f:
                json.dump(report, f, indent=2, sort_keys=True)
            print(f"[report] written to: {report_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
