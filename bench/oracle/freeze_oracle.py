#!/usr/bin/env python3
"""Create oracle snapshot fingerprint for TPC-H v2 data.

Usage: python3 freeze_oracle.py
"""
import json
import os
import subprocess
import sys
from pathlib import Path
from datetime import datetime, timezone

SNAPSHOT_DIR = Path("/Users/liying/workspace/dev/yinglichina163/sqlrustgo/bench/oracle/tpch_sf01_snapshot_v2")
SNAPSHOT_DIR.mkdir(parents=True, exist_ok=True)

TABLES = ["region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem"]
PG_ENV = {"PGPASSWORD": "test123", "PATH": "/opt/homebrew/opt/postgresql@16/bin:/usr/bin:/bin"}
PSQL = "/opt/homebrew/opt/postgresql@16/bin/psql"


def psql_strict(query: str, timeout: int = 60) -> str:
    """Run psql with strict flags (P0-3 from user 2026-06-07 feedback):
    -X          no .psqlrc (deterministic)
    -v ON_ERROR_STOP=1   stop on error
    -q          quiet (no startup message)
    -t          tuples only (no header)
    -A          unaligned output
    -c          command
    """
    cmd = [PSQL, "-U", "liying", "-d", "tpch_test",
           "-X", "-v", "ON_ERROR_STOP=1", "-q", "-t", "-A",
           "-c", query]
    r = subprocess.run(cmd, env=PG_ENV, capture_output=True, text=True, timeout=timeout)
    if r.returncode != 0:
        raise RuntimeError(f"psql failed for query {query[:50]!r}: {r.stderr}")
    return r.stdout.strip()


def main():
    table_counts = {}
    table_hashes = {}

    for tbl in TABLES:
        count = int(psql_strict(f"SELECT count(*) FROM {tbl}"))
        table_counts[tbl] = count
        # Hash a sample of 1000 rows for fingerprint
        hash_sql = (
            f"SELECT md5(string_agg(t::text, '|' ORDER BY t::text)) "
            f"FROM (SELECT * FROM {tbl} LIMIT 1000) t"
        )
        sample_hash = psql_strict(hash_sql)[:32]
        table_hashes[tbl] = sample_hash
        print(f"  {tbl:12} count={count:>6}  sample_md5={sample_hash}", file=sys.stderr)

    meta = {
        "snapshot_name": "tpch_sf01_snapshot_v2",
        "created_at": datetime.now(timezone.utc).isoformat(),
        "data_source": "/tmp/tpch_sf01_v2 (regenerated with discount fix)",
        "frozen": True,
        "freeze_rule": "DO NOT mutate PG while this snapshot is active. Use reads only.",
        "psql_strict_flags": [
            "-X",            # no .psqlrc
            "-v", "ON_ERROR_STOP=1",
            "-q",            # quiet
            "-t",            # tuples-only
            "-A",            # unaligned
        ],
        "tables": {
            tbl: {
                "row_count": table_counts[tbl],
                "sample_md5": table_hashes[tbl],
            }
            for tbl in TABLES
        },
        "row_semantics": {
            "scalar_aggregate": "1 row (with NULL value if no input rows)",
            "group_by": "0+ rows matching the data",
            "exists_subquery": "0 or 1 row (boolean)",
            "empty_relation": "0 rows",
        },
        "comparator_states": {
            "PASS": "semantic match (accounting for NULL/empty semantics)",
            "FAIL": "semantic mismatch with PG oracle",
            "TIMEOUT": "engine did not complete in time (non-verifiable)",
            "DATA_LIMITATION": "PG returns 0 rows; cannot verify engine without non-zero reference",
            "ENGINE_ISSUE": "PG returns N>0; engine returns 0 (missing data)",
        },
    }
    meta_path = SNAPSHOT_DIR / "meta.json"
    with open(meta_path, "w") as f:
        json.dump(meta, f, indent=2, sort_keys=True)
    print(f"\n[OK] meta.json written to: {meta_path}", file=sys.stderr)
    print(f"     Use this fingerprint to detect future PG mutations.", file=sys.stderr)


if __name__ == "__main__":
    main()
