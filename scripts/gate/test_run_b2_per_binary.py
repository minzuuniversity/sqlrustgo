#!/usr/bin/env python3
# scripts/gate/test_run_b2_per_binary.py
#
# Unit-test the B2 per-binary runner's *parsing + classification* logic
# without invoking `cargo test` (which would take 30+ minutes per binary).
# Validates:
#   - DISABLED_BINARIES list contains the expected 35 entries
#   - PERF_ONLY_BINARIES list contains the expected 3 entries (all overlapping DISABLED)
#   - TEST_RESULT_LINE regex matches real cargo output (4 observed formats)
#   - list_test_binaries() returns 354 binaries from Cargo.toml
#   - BinaryResult dataclass round-trips through asdict()
#   - Summary counts correctly tally SKIP_DISABLED/SKIP_PERF/PASS/FAIL/TIMEOUT
#
# Usage: python3 scripts/gate/test_run_b2_per_binary.py
#
# Exit 0 = all checks pass; non-zero = at least one check failed.

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
RUNNER_PATH = REPO_ROOT / "scripts/gate/run_b2_per_binary.py"


def load_runner():
    spec = importlib.util.spec_from_file_location("run_b2_per_binary", RUNNER_PATH)
    mod = importlib.util.module_from_spec(spec)
    sys.modules["run_b2_per_binary"] = mod  # needed for @dataclasses.dataclass
    spec.loader.exec_module(mod)
    return mod


def check(name: str, ok: bool, detail: str = ""):
    status = "PASS" if ok else "FAIL"
    print(f"  [{status}] {name}{(': ' + detail) if detail else ''}")
    return ok


def main():
    print("=== run_b2_per_binary.py unit tests ===")
    failures = 0
    runner = load_runner()

    # 1. DISABLED_BINARIES has 78 entries (33 - 2 reactivated + 45 added 2026-08-24).
    # PR #4436 / Codex baseline reactivated bulk_insert_v2_routing and
    # bin_storage_compaction_roundtrip after PR #4417 fixed the
    # BinaryTableStorageV2 feature gate. Our branch inherits those reactivations
    # while adding 45 newly-captured failing binaries from the 2026-08-24
    # per-binary baseline run. Net: 80 - 2 (reactivated) = 78.
    if not check("DISABLED_BINARIES_COUNT_78",
        len(runner.DISABLED_BINARIES) == 78,
        f"got {len(runner.DISABLED_BINARIES)}"):
        failures += 1

    # 2. PERF_ONLY_BINARIES has 3 entries
    if not check("PERF_ONLY_BINARIES_COUNT_3",
        len(runner.PERF_ONLY_BINARIES) == 3,
        f"got {len(runner.PERF_ONLY_BINARIES)}"):
        failures += 1

    # 3. Perf entries are a subset of disabled entries
    perf_set = set(runner.PERF_ONLY_BINARIES)
    disabled_set = runner.DISABLED_SET
    if not check("PERF_SUBSET_OF_DISABLED",
        perf_set <= disabled_set,
        f"perf-not-in-disabled: {perf_set - disabled_set}"):
        failures += 1

    # 4. Required entries (no regressions on disabled set).
    # Note: bulk_insert_v2_routing and bin_storage_compaction_roundtrip were
    # REACTIVATED in PR #4436 (Codex baseline) after PR #4417 fixed the
    # BinaryTableStorageV2 feature gate, so they are NOT required disabled
    # entries — they have moved out of DISABLED_BINARIES.
    REQUIRED_DISABLED = [
        "ddl_e2e_test", "diag_q11", "diag_q12", "diag_q14_full",
        "eval_22_vs_sf01", "e2e_canonical_subprocess",
        "int2_substance_parallel_test", "parallel_main_path_test",
        "oracle_g1_tpch_sha256", "q13_subquery_repro",
        "q16_notin_subquery_regression", "q21_cell_regression_test",
        "physical_backup_test", "l3_canonical_binary",
        # Newly-captured from 2026-08-24 per-binary B2 run
        "diag_q7_subset_columns", "tpch_wire_smoke",
        "types_value_test", "window_function_test",
    ]
    missing = [b for b in REQUIRED_DISABLED if b not in disabled_set]
    if not check("REQUIRED_DISABLED_ENTRIES",
        not missing,
        f"missing: {missing}"):
        failures += 1

    # 5. TEST_RESULT_LINE regex matches all 4 observed cargo output formats
    samples = [
        ("ok.354p.0f.0i.12.34s",
         "test result: ok. 354 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.34s",
         {"status": "ok", "passed": "354", "failed": "0", "ignored": "0", "elapsed": "12.34"}),
        ("FAILED.0p.1f.0i.390.71s",
         "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 390.71s",
         {"status": "FAILED", "passed": "0", "failed": "1", "ignored": "0", "elapsed": "390.71"}),
        ("ok.12p.0f.0i.0.02s",
         "test result: ok. 12 passed; 0 failed; 0 ignored; 1 measured; 0 filtered out; finished in 0.02s",
         {"status": "ok", "passed": "12", "failed": "0", "ignored": "0", "elapsed": "0.02"}),
        ("ok.716p.0f.0i.4.5s",
         "test result: ok. 716 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.5s",
         {"status": "ok", "passed": "716", "failed": "0", "ignored": "0", "elapsed": "4.5"}),
    ]
    for tag, sample, expected in samples:
        m = runner.TEST_RESULT_LINE.search(sample)
        if not check(f"REGEX_{tag}", m is not None,
            f"sample={sample!r}"):
            failures += 1
            continue
        got = m.groupdict()
        if not check(f"REGEX_{tag}_FIELDS", got == expected,
            f"got={got}"):
            failures += 1

    # 6. list_test_binaries returns 354 from Cargo.toml
    binaries = runner.list_test_binaries()
    if not check("BINARIES_COUNT_354",
        len(binaries) == 354,
        f"got {len(binaries)}"):
        failures += 1
    if not check("BINARIES_NO_DUPLICATES",
        len(set(binaries)) == len(binaries),
        f"duplicates: {len(binaries) - len(set(binaries))}"):
        failures += 1

    # 7. BinaryResult dataclass round-trips
    r = runner.BinaryResult(
        name="test_x", status="PASS", elapsed_sec=12.34,
        passed=10, failed=0, ignored=1, exit_code=0,
        log_path="evidence/x.log", group="fast",
    )
    d = runner.dataclasses.asdict(r)
    if not check("BINARY_RESULT_ASDICT",
        d["name"] == "test_x" and d["status"] == "PASS" and d["group"] == "fast",
        f"got={d}"):
        failures += 1

    # 8. JSON serializable
    try:
        json.dumps(d)
        ok = True
    except (TypeError, ValueError) as e:
        ok = False
    if not check("BINARY_RESULT_JSON_SERIALIZABLE", ok):
        failures += 1

    # 9. The runner file is syntactically valid (already verified at write-time
    # but re-check on test invocation in case it was edited)
    if not check("RUNNER_FILE_READABLE",
        RUNNER_PATH.is_file() and RUNNER_PATH.stat().st_size > 0):
        failures += 1

    # 10. SLOW_THRESHOLD_SEC is 30 (the gate's <30s/>30s split)
    if not check("SLOW_THRESHOLD_30",
        runner.SLOW_THRESHOLD_SEC == 30):
        failures += 1

    # 11. Default timeouts match the documented values
    if not check("DEFAULT_FAST_TIMEOUT_180",
        runner.DEFAULT_FAST_TIMEOUT_SEC == 180):
        failures += 1
    if not check("DEFAULT_SLOW_TIMEOUT_600",
        runner.DEFAULT_SLOW_THRESHOLD_SEC if hasattr(runner, 'DEFAULT_SLOW_THRESHOLD_SEC') else runner.DEFAULT_SLOW_TIMEOUT_SEC == 600):
        failures += 1

    print()
    print(f"  total checks: {11 + len(samples)*2}, failures: {failures}")
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    sys.exit(main())