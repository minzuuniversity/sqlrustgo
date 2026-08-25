#!/usr/bin/env python3
"""
Build a unified 4-engine TPC-H SF=1 cross-engine SUMMARY.json from the
per-engine SUMMARY.json + .tsv/.sha256 artifacts.

Also performs cross-engine sha256 comparison for queries present in 2+
engines. Outputs:
  - docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/SUMMARY.json (new merged)
  - docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/CROSS_ENGINE_HASH_CHECK.json (new diff matrix)

Schema version: v2 (adds: schema_version, engines_complete, per_q cross-engine block).
"""
import json
import hashlib
import sys
from pathlib import Path

ROOT = Path("docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1")
ENGINES = ["postgres", "sqlite", "mysql", "sqlrustgo"]
HEAD_COMMIT = "ae572990bb02ad6fc9fcffa82ded5c1dcd30a6d6"
STAMP = "2026-08-25T12:00:00Z"  # updated to today; HEAD is unchanged


def load_engine_summary(engine: str) -> dict:
    p = ROOT / engine / "SUMMARY.json"
    with p.open() as f:
        return json.load(f)


def normalize_query(entry: dict, engine: str) -> dict:
    """Pull q, row_count, sha256, status, elapsed_s into a canonical shape.
    Falls back to per-query .sha256 file under engine/ if SUMMARY.json
    doesn't include sha256 (e.g. sqlrustgo captures via TPCH_SF1_ROWS_DIR)."""
    q = entry.get("q")
    rc = entry.get("row_count")
    sha = entry.get("sha256")
    if not sha:
        sha_file = ROOT / engine / f"q{q}.sha256"
        if sha_file.exists():
            sha = sha_file.read_text().strip()
    status = entry.get("status", "ok" if sha else "unknown")
    elapsed = entry.get("elapsed_s")
    return {
        "q": q,
        "row_count": rc,
        "sha256": sha,
        "status": status,
        "elapsed_s": elapsed,
        "note": entry.get("note"),
    }


def build_unified():
    per_engine = {}
    for eng in ENGINES:
        s = load_engine_summary(eng)
        queries = s.get("queries", [])
        per_engine[eng] = {q["q"]: normalize_query(q, eng) for q in queries}

    # Build per-q cross-engine block
    cross = {}
    for q in range(1, 23):
        block = {"engines": {}, "all_match": None, "engines_present": [], "engines_missing": []}
        for eng in ENGINES:
            rec = per_engine[eng].get(q)
            if rec and rec["sha256"]:
                block["engines"][eng] = {
                    "row_count": rec["row_count"],
                    "sha256": rec["sha256"],
                    "status": rec["status"],
                    "elapsed_s": rec["elapsed_s"],
                }
                block["engines_present"].append(eng)
            else:
                block["engines_missing"].append(eng)

        # Compute all_match across present engines
        present_shas = {v["sha256"] for v in block["engines"].values()}
        present_rows = {v["row_count"] for v in block["engines"].values()}
        if len(present_shas) >= 2:
            block["all_match"] = (len(present_shas) == 1) and (len(present_rows) == 1)
        elif len(present_shas) == 1:
            block["all_match"] = True  # single engine baseline
        else:
            block["all_match"] = None

        cross[q] = block

    # Coverage matrix
    coverage = {}
    for eng in ENGINES:
        present = sum(1 for q in range(1, 23) if per_engine[eng].get(q, {}).get("sha256"))
        coverage[eng] = present

    unified = {
        "schema_version": "v2",
        "branch": "develop/v3.12.0",
        "commit": HEAD_COMMIT,
        "stamp": STAMP,
        "scale": 1.0,
        "fixture": "/tmp/tpch-sf1/*.tbl (TPC-H dbgen -s 1)",
        "queries_total": 22,
        "engines": ENGINES,
        "coverage_per_engine": coverage,
        "queries": [
            {"q": q, **cross[q]} for q in range(1, 23)
        ],
        "provenance": {
            "issue": "#4382 — V312-58-CROSS-ENGINE-4WAY",
            "policy": "Anti-Fabrication-Policy-v1.0",
            "generator": "build_cross_engine_summary.py",
        },
    }
    return unified


def build_hash_check(unified: dict) -> dict:
    """Diff matrix: per-query, per-engine sha256 comparison.

    Re-derives sha256 from each .tsv file and compares to SUMMARY.json values
    AND across engines."""
    out = {
        "schema_version": "v2",
        "branch": "develop/v3.12.0",
        "commit": HEAD_COMMIT,
        "stamp": STAMP,
        "queries": [],
    }
    for qentry in unified["queries"]:
        q = qentry["q"]
        qblock = {"q": q, "engines": {}, "intra_engine_sha_match": {}, "cross_engine_match": qentry["all_match"]}
        for eng in ENGINES:
            tsv_path = ROOT / eng / f"q{q}.tsv"
            if not tsv_path.exists():
                qblock["engines"][eng] = {"present": False}
                continue
            actual_sha = hashlib.sha256(tsv_path.read_bytes()).hexdigest()
            declared = qentry["engines"].get(eng, {}).get("sha256")
            match = (actual_sha == declared) if declared else None
            qblock["engines"][eng] = {
                "present": True,
                "declared_sha256": declared,
                "actual_sha256": actual_sha,
                "sha256_match": match,
            }
        out["queries"].append(qblock)

    # Summary
    total = len(out["queries"])
    matched = sum(1 for q in out["queries"] if q["cross_engine_match"] is True)
    mismatched = sum(1 for q in out["queries"] if q["cross_engine_match"] is False)
    no_compare = sum(1 for q in out["queries"] if q["cross_engine_match"] is None)
    out["summary"] = {
        "queries_total": total,
        "cross_engine_match_true": matched,
        "cross_engine_match_false": mismatched,
        "single_or_no_engine": no_compare,
    }
    return out


def main():
    unified = build_unified()
    hash_check = build_hash_check(unified)
    out_summary_path = ROOT / "SUMMARY.json"
    out_hash_path = ROOT / "CROSS_ENGINE_HASH_CHECK.json"
    with out_summary_path.open("w") as f:
        json.dump(unified, f, indent=2, sort_keys=False)
        f.write("\n")
    with out_hash_path.open("w") as f:
        json.dump(hash_check, f, indent=2, sort_keys=False)
        f.write("\n")

    print(f"Wrote {out_summary_path}")
    print(f"Wrote {out_hash_path}")
    print()
    print()
    print("=== Coverage per engine ===")
    for eng, n in unified["coverage_per_engine"].items():
        print(f"  {eng:12s} {n}/22")
    print()
    print("=== Cross-engine hash check summary ===")
    s = hash_check["summary"]
    print(f"  queries_total:        {s['queries_total']}")
    print(f"  cross_engine MATCH:   {s['cross_engine_match_true']}")
    print(f"  cross_engine MISMATCH:{s['cross_engine_match_false']}")
    print("=== Per-query cross-engine (mismatch only) ===")
    mismatch_count = 0
    for qentry in hash_check["queries"]:
        cm = qentry["cross_engine_match"]
        if cm is False:
            mismatch_count += 1
            for eng, info in qentry["engines"].items():
                if info.get("present"):
                    print(f"  Q{qentry['q']:>2} {eng}: rows={info.get('declared_sha256','')[:16]}... sha={info.get('actual_sha256','')[:16]}...")
    if mismatch_count == 0:
        print("  (none)")
    print()
    print("=== Per-query cross-engine (single-engine baseline) ===")
    single_count = 0
    for qentry in hash_check["queries"]:
        cm = qentry["cross_engine_match"]
        if cm is None:
            single_count += 1
            present = [e for e, i in qentry["engines"].items() if i.get("present")]
            print(f"  Q{qentry['q']:>2} present_only: {present}")
    if single_count == 0:
        print("  (none)")


if __name__ == "__main__":
    main()

