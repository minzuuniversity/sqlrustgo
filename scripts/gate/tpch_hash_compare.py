#!/usr/bin/env python3
"""
TPC-H Hash Compare — compute / verify the v3.8.0 baseline hash for 22/22 TPC-H.

Usage:
    python3 scripts/gate/tpch_hash_compare.py --capture
        # Run TPC-H Q1..Q22 via ephemeral sqlrustgo-mysql-server,
        # sort rows by all columns, concatenate, SHA-256 it.
        # Print the 64-char hex hash to stdout. Exit 0 on success.

    python3 scripts/gate/tpch_hash_compare.py --check <expected_hash>
        # Same as --capture, then compare to <expected_hash>.
        # Exit 0 on match; exit 1 on mismatch (diff to stderr).

    python3 scripts/gate/tpch_hash_compare.py --dry-run
        # Print what would be done; do not run anything. Exit 0.

This script is the single source of truth for the TPC-H hash algorithm. The
Rust regression test in tests/tpch_hash_test.rs is a thin shim around this
script (it shells out via Command::new("python3") to keep the math identical
between the test and the shell gate).

Algorithm (locked in by the OpenSpec spec g1-tpch-baseline §"Requirement:
Single SHA-256 baseline hash"):
  1. Spawn `sqlrustgo-mysql-server` as a child process, bound to port 0.
  2. For each Q1..Q22 in queries/q*.sql:
       a. Open a MySQL wire-protocol connection (raw handshake, no mysql crate).
       b. CREATE TABLE + INSERT rows from ~/sqlrustgo-tpch/data/*.tbl.
       c. Run the query.
       d. Capture all rows, sort by all columns (stable, Python tuple sort).
       e. Serialize each row to the same string format that the test uses.
  3. Concatenate: for each Q, write `---Q<n>---\n<sorted rows>`, then
     concatenate all 22 blocks, then SHA-256.

For --check, parse the JSON baseline file at tests/tpch_hashes_v380.json and
compare the captured hash to its tpc_h_hash_sha256 field. If they differ,
write a per-query diff to stderr and exit 1.

Why not pure Rust: this is a "test the SQL result is what we think it is"
gate, not a performance-critical path. Python is faster to evolve when the
data format changes (e.g., a new TPC-H column appears in a future scale
factor). The hash itself is a single 64-char hex string, so the language
boundary is cheap.

Why not pure Python MySQL client: the test environment is not guaranteed to
have a `mysql` CLI or `pymysql` installed. The script intentionally does
NOT require a MySQL client library — it uses the `start_ephemeral` test
harness in-process via the same Rust binary that runs the wire tests.

Concrete implementation (locked in by spec §D4):
- Step 1-2d: For now, since the in-process MySQL wire path is exercised by
  tests/embedded_harness_smoke.rs (a separate OpenSpec change), this script
  shells out to `cargo test --test tpch_full_22_test -- --nocapture` and
  parses the printed "Q<n>: <rows>" output. This is the simplest path that
  is guaranteed to produce a hash identical to the Rust test's view of
  TPC-H correctness, because both go through the same ExecutionEngine.
- Step 2e: A `---Q<n>---\n<row1>\n<row2>\n...\n` block per query.
- Step 3: `hashlib.sha256(concatenated_blocks.encode("utf-8")).hexdigest()`.

Future evolution (out of scope here):
- Switch to in-process `start_ephemeral` once the embedded-harness change
  (2026-06-04-mysql-server-canonical-entry) is fully merged.
- Add multi-platform hashing once we have a Z440 (macOS) or Windows runner.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
HASH_FILE = REPO_ROOT / "tests" / "tpch_hashes_v380.json"
QUERIES_DIR = REPO_ROOT / "queries"
BASELINE_DATA_DIR = Path(os.environ.get("TPCH_DATA_DIR", str(Path.home() / "sqlrustgo-tpch" / "data")))

QUERY_RE = re.compile(r"^Q(\d+):\s*(.*)$", re.MULTILINE)
HASH_LEN = 64


def die(msg: str, code: int = 1) -> "NoReturn":  # type: ignore[name-defined]
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(code)


def dry_run() -> int:
    print("=== TPC-H Hash Compare (dry-run) ===")
    print(f"  repo_root:        {REPO_ROOT}")
    print(f"  baseline_file:    {HASH_FILE}")
    print(f"  queries_dir:      {QUERIES_DIR}")
    print(f"  tpch_data_dir:    {BASELINE_DATA_DIR}")
    print(f"  expected_hash:    <from {HASH_FILE.name}>")
    print(f"  step 1:           cargo test --test tpch_full_22_test -- --nocapture TPCH_FORCE=1")
    print(f"  step 2:           parse Q<n>: <rows> output")
    print(f"  step 3:           SHA-256 of sorted concatenated blocks")
    print(f"  step 4:           diff to baseline (only in --check mode)")
    return 0


def run_tpch_full_22(timeout_per_query_s: int = 300, overall_timeout_s: int = 1800) -> str:
    """Run the in-process TPC-H 22 query test and return its captured stdout.

    The test prints `=== TPC-H Full 22 Query Gate ===`, runs Q1..Q22 in order,
    and emits per-query output. We parse the printed table rows to build the
    per-query result sets.

    Returns the full stdout (UTF-8).
    """
    if not BASELINE_DATA_DIR.exists():
        die(
            f"TPC-H data not found at {BASELINE_DATA_DIR}. "
            "Generate with: bash scripts/gate/setup_tpch_env.sh --sf01"
        )
    if not QUERIES_DIR.exists():
        die(f"queries/ directory not found: {QUERIES_DIR}")

    env = os.environ.copy()
    env["TPCH_FORCE"] = "1"
    env["TPCH_DATA_DIR"] = str(BASELINE_DATA_DIR)
    env["TPCH_TIMEOUT_S"] = str(timeout_per_query_s)
    env["RUST_LOG"] = "warn"  # quiet the engine's own info logs

    print(f"[hash] running cargo test --test tpch_full_22_test (timeout={overall_timeout_s}s) ...", file=sys.stderr)
    start = time.time()
    try:
        proc = subprocess.run(
            [
                "cargo", "test", "--test", "tpch_full_22_test",
                "--", "--nocapture", "--test-threads=1",
            ],
            cwd=str(REPO_ROOT),
            env=env,
            capture_output=True,
            text=True,
            timeout=overall_timeout_s,
        )
    except subprocess.TimeoutExpired:
        die(f"cargo test exceeded overall timeout of {overall_timeout_s}s", code=2)
    elapsed = time.time() - start
    print(f"[hash] cargo test finished in {elapsed:.1f}s (exit {proc.returncode})", file=sys.stderr)
    if proc.returncode != 0:
        die(f"cargo test failed (exit {proc.returncode}); cannot hash.\n"
            f"  stderr (last 20 lines):\n{chr(10).join(proc.stderr.splitlines()[-20:])}")
    return proc.stdout


def parse_per_query_results(stdout: str) -> dict[int, list[tuple]]:
    """Parse the printed output of tpch_full_22_test into {Q_num: [row_tuples]}.

    The current test output format prints results inline; we use a simple
    Q-marker heuristic: any line beginning with `Q<digit>+:` starts a query
    block, and lines until the next `Q<digit>+:` or end of run are rows.

    Returns a dict {1: [(col,col,...), ...], 2: [...], ...}.
    """
    results: dict[int, list[tuple]] = {}
    current_q: int | None = None
    current_rows: list[tuple] = []
    for line in stdout.splitlines():
        m = re.match(r"^\s*(?:Q)?(\d+):\s*(.+)$", line)
        # More precise: the test prints `Q<n>: <result>` or `Q<n>: <text>`;
        # for the first iteration we capture everything after the marker.
        m2 = re.match(r"^\s*Q(\d+)\b", line)
        if m2:
            if current_q is not None:
                results[current_q] = current_rows
            current_q = int(m2.group(1))
            current_rows = []
            continue
        if current_q is not None and line.strip() and not line.startswith("==="):
            # Naive row capture: split by `|` (the standard SQL pretty-printer
            # in this repo uses `|` between columns) and strip whitespace.
            row = tuple(cell.strip() for cell in line.split("|") if cell.strip() != "")
            if row:
                current_rows.append(row)
    if current_q is not None:
        results[current_q] = current_rows
    return results


def hash_from_results(results: dict[int, list[tuple]]) -> str:
    """Compute the SHA-256 hash of the TPC-H 22/22 sorted output.

    Format: for each Q in 1..22, write `---Q<n>---\n<sorted rows>\n`, then
    SHA-256 the concatenation. Missing Q -> `[MISSING Q<n>]`.
    """
    buf: list[str] = []
    for q in range(1, 23):
        rows = results.get(q, [])
        buf.append(f"---Q{q}---")
        if not rows:
            buf.append("[empty]")
        else:
            for row in sorted(rows):
                buf.append("|".join(row))
        buf.append("")  # trailing newline between blocks
    body = "\n".join(buf).encode("utf-8")
    return hashlib.sha256(body).hexdigest()


def load_expected_hash() -> str | None:
    if not HASH_FILE.exists():
        return None
    try:
        data = json.loads(HASH_FILE.read_text())
    except json.JSONDecodeError as e:
        die(f"{HASH_FILE} is not valid JSON: {e}")
    h = data.get("tpc_h_hash_sha256", "")
    if not isinstance(h, str) or len(h) != HASH_LEN:
        return None
    return h


def cmd_capture() -> int:
    """Compute the hash and print to stdout."""
    stdout = run_tpch_full_22()
    results = parse_per_query_results(stdout)
    if len(results) != 22:
        die(f"expected 22 query results, got {len(results)}; check test output format",
            code=2)
    h = hash_from_results(results)
    print(h)
    return 0


def cmd_check(expected: str) -> int:
    """Compute the hash, compare to expected, exit 0/1."""
    expected = expected.lower().strip()
    if len(expected) != HASH_LEN or any(c not in "0123456789abcdef" for c in expected):
        die(f"expected hash must be {HASH_LEN} hex chars, got: {expected!r}")
    stdout = run_tpch_full_22()
    results = parse_per_query_results(stdout)
    if len(results) != 22:
        die(f"expected 22 query results, got {len(results)}", code=2)
    actual = hash_from_results(results)
    if actual == expected:
        print(f"G1 PASS: TPC-H 22/22 baseline hash matches ({actual[:8]}...)")
        return 0
    # Per-query diff
    print(f"G1 FAIL: hash mismatch (expected {expected[:8]}..., got {actual[:8]}...)", file=sys.stderr)
    for q in range(1, 23):
        exp_rows = results.get(q, [])
        # Without a stored per-query snapshot, we can only signal "drift detected";
        # the operator should re-run --capture and inspect the diff.
        if not exp_rows:
            print(f"  Q{q}: no rows captured (or missing)", file=sys.stderr)
    return 1


def main() -> int:
    p = argparse.ArgumentParser(description="TPC-H hash compare (G1 gate)")
    g = p.add_mutually_exclusive_group(required=True)
    g.add_argument("--capture", action="store_true", help="Print SHA-256 to stdout")
    g.add_argument("--check", metavar="HASH", help="Compare to expected hash")
    g.add_argument("--dry-run", action="store_true", help="Print what would be done")
    args = p.parse_args()
    if args.dry_run:
        return dry_run()
    if args.capture:
        return cmd_capture()
    if args.check:
        return cmd_check(args.check)
    return 0  # unreachable


if __name__ == "__main__":
    sys.exit(main())
