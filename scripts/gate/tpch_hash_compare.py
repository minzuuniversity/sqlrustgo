#!/usr/bin/env python3
"""
TPC-H Hash Compare — compute / verify the v3.8.0 baseline hash for 22/22 TPC-H.

Usage:
    python3 scripts/gate/tpch_hash_compare.py --capture
        # Run the Rust tpch_hash_test, parse the printed hash.
        # Print the 64-char hex hash to stdout. Exit 0 on success.

    python3 scripts/gate/tpch_hash_compare.py --check <expected_hash>
        # Same as --capture, then compare to <expected_hash>.
        # Exit 0 on match; exit 1 on mismatch (diff to stderr).

    python3 scripts/gate/tpch_hash_compare.py --dry-run
        # Print what would be done; do not run anything. Exit 0.

This script orchestrates the Rust test tests/tpch_hash_test.rs which is the
actual hash computation. The Rust test uses in-process start_sf001() +
MySqlTestClient to run all 22 TPC-H queries, sort results, and SHA-256 hash.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
HASH_FILE = REPO_ROOT / "tests" / "tpch_hashes_v380.json"
HASH_LEN = 64

HASH_RE = re.compile(rb"G1 PASS: TPC-H 22/22 baseline hash matches \(([a-f0-9]+)\)")


def die(msg: str, code: int = 1) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(code)


def dry_run() -> int:
    print("=== TPC-H Hash Compare (dry-run) ===")
    print(f"  repo_root:    {REPO_ROOT}")
    print(f"  hash_file:    {HASH_FILE}")
    print(f"  step 1:       cargo test --test tpch_hash_test -- --nocapture")
    print(f"  step 2:       parse hash from test output")
    print(f"  step 3:       compare to baseline (--check only)")
    return 0


def run_tpch_hash_test() -> bytes:
    env = os.environ.copy()
    env["RUST_LOG"] = "warn"
    proc = subprocess.run(
        ["cargo", "test", "--test", "tpch_hash_test", "--", "--nocapture"],
        cwd=str(REPO_ROOT),
        env=env,
        capture_output=True,
        timeout=600,
    )
    return proc.stdout + proc.stderr


def parse_hash(output: bytes) -> str:
    m = HASH_RE.search(output)
    if m:
        return m.group(1).decode()
    die("cannot find hash in test output", code=2)


def cmd_capture() -> int:
    output = run_tpch_hash_test()
    h = parse_hash(output)
    print(h)
    return 0


def cmd_check(expected: str) -> int:
    expected = expected.lower().strip()
    if len(expected) != HASH_LEN or any(c not in "0123456789abcdef" for c in expected):
        die(f"expected hash must be {HASH_LEN} hex chars")

    output = run_tpch_hash_test()
    actual = parse_hash(output)
    if actual == expected:
        print(f"G1 PASS: TPC-H 22/22 baseline hash matches ({actual[:8]}...)")
        return 0

    print(f"G1 FAIL: hash mismatch", file=sys.stderr)
    print(f"  expected: {expected}", file=sys.stderr)
    print(f"  actual:   {actual}", file=sys.stderr)
    print(file=sys.stderr)
    print(f"  To update the baseline:", file=sys.stderr)
    print(f"    python3 scripts/gate/tpch_hash_compare.py --capture", file=sys.stderr)
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
