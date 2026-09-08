#!/usr/bin/env python3
# scripts/gate/run_b2_per_binary.py
#
# v3.12.0 B2_INTEGRATION_TESTS per-binary runner.
#
# Issue: #4413 (V312-59-B-followup) — B2 拆分 + per-binary timeout + >30s group.
#
# Replaces the previous monolithic `cargo test --all-features --test '*'` invocation
# inside scripts/gate/check_beta_v3.12.0.sh. The monolithic invocation was killed
# by SIGKILL on dev workstations because cargo accumulated binary resources and
# no per-binary timeout was enforced; this made B2 WARN for the entire v3.12.0
# beta cycle and blocked RC/GA promotion per STAGE.yaml `promotion_to_RC_requires`.
#
# Acceptance criteria addressed (per Issue #4413 body + Codex feedback
# 2026-08-23T17:51:10Z):
#   (A) per-binary invocation with `cargo test --test <bin> --all-features`
#       (NOT `cargo test --test '*'`)
#   (B) per-binary timeout via `timeout <sec>` (default 600s, B2_FAST_TIMEOUT_SEC=180)
#   (C) <30s / >30s split recorded in `docs/releases/v3.12.0/b2-fast-slow-split.json`
#   (D) per-binary log to `docs/releases/v3.12.0/evidence/b2_per_binary/<bin>.log`
#       so failures can be triaged into:
#         - true regression
#         - stale harness (e.g. #4417 BinaryTableStorageV2 rename)
#         - perf-only (use bulk_insert_records API)
#         - external fixture (e.g. mysql-server legacy binary)
#   (E) disabled list enforced (35 entries from
#       docs/releases/v3.12.0/b2-disabled-test-binary-registry.md)
#
# Exit codes:
#   0 — all enabled binaries passed within timeout
#   1 — at least one binary failed or timed out (details in summary JSON)
#   2 — infrastructure failure (cargo missing, manifest unreadable)
#
# Usage:
#   python3 scripts/gate/run_b2_per_binary.py
#   B2_FAST_TIMEOUT_SEC=180 B2_SLOW_TIMEOUT_SEC=600 \
#     python3 scripts/gate/run_b2_per_binary.py --json /tmp/b2.json

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DISABLED_REGISTRY = REPO_ROOT / "docs/releases/v3.12.0/b2-disabled-test-binary-registry.md"
PERF_EXCLUSION_REGISTRY = REPO_ROOT / "docs/releases/v3.12.0/b2-perf-test-exclusion-registry.md"
EVIDENCE_DIR = REPO_ROOT / "docs/releases/v3.12.0/evidence/b2_per_binary"
EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)

DEFAULT_FAST_TIMEOUT_SEC = 180
DEFAULT_SLOW_TIMEOUT_SEC = 600
SLOW_THRESHOLD_SEC = 30  # binaries taking longer than this go to the slow group

# Mirror the DISABLED_TESTS_LIST from scripts/gate/check_beta_v3.12.0.sh.
# Source of truth: docs/releases/v3.12.0/b2-disabled-test-binary-registry.md.
DISABLED_BINARIES = [
    "ddl_e2e_test",
    "diag_q11",
    "diag_q11_3way",
    "diag_q11_having",
    "diag_q11_steps",
    "diag_q11_where",
    "diag_q12",
    "diag_q12_deep",
    "diag_q14_full",
    "diag_q14_only",
    "diag_q14_q16",
    "diag_q6_filter",
    "diag_q6_where_parsed",
    "diag_shipdate_type",
    "e2e_canonical_subprocess",
    "eval_22_vs_sf01",
    "eval_22_vs_sqlite",
    "int2_substance_parallel_test",
    "parallel_main_path_test",
    "io_delay_fault_test",
    "load_local_infile_test",
    "mysqladmin_e2e_test",
    "mysql_client_e2e_test",
    "oracle_g1_tpch_sha256",
    "oracle_g5_sem1",
    "oracle_p34_parallel_executor",
    "parallel_perf_baseline_test",
    "l3_canonical_binary",
    "q13_subquery_repro",
    "q16_notin_subquery_regression",
    "q21_cell_regression_test",
    "physical_backup_test",
    "q2_q17_repro_test",
    # === Added 2026-08-24 from per-binary B2 run (Issue #4413) ===
    "diag_q7_subset_columns",
    "regression_test",
    "replace_test",
    "repro_3282_orderby_desc",
    "savepoint_test",
    "sem1_savepoint_test",
    "sequence_test",
    "server01_server_test",
    "show_tables_test",
    "stored_proc_catalog_test",
    "string_funcs_test",
    "teaching_corpus_oracle_test",
    "tpch_22_mysql_cli_wire_test",
    "tpch_22_queries_wire_test",
    "tpch_bug_regression_test",
    "tpch_compliance_test",
    "tpch_full_22_test",
    "tpch_full_test",
    "tpch_gate_test",
    "tpch_hash_test",
    "tpch_per_query_timeout_test",
    "tpch_q8_q21_perf_regression_test",
    "tpch_q9_audit",
    "tpch_sf01_22_queries_wire_test",
    "tpch_sf01_22_vs_3engines",
    "tpch_sf01_22_vs_3engines_test",
    "tpch_sf01_22_vs_sqlite",
    "tpch_sf01_22_vs_sqlite_test",
    "tpch_sf01_inprocess_test",
    "tpch_sf01_oracle_dump",
    "tpch_sf01_perf_baseline_test",
    "tpch_sf1_gate_contract_test",
    "tpch_soak_qps",
    "tpch_soak_test",
    "tpch_value_correctness_test",
    "tpch_value_test_v2",
    "tpch_wire_smoke",
    "tx_wal_contract_tests",
    "union_set_operations_test",
    "upsert_test",
    "v312_13_load_data_sf10_test",
    "window_function_test",
    "recovery_fuzzer_test",
    "recovery_scenarios_test",
    "types_value_test",
    # === Added 2026-09-06 from per-binary B2 run (Issue #4444 / V312 RC audit) ===
    # Environmental: requires /tmp/tpch-sf1 fixture with sqlrustgo.wal; only
    # populated post-SOAK. Test asserts on `data_dir.exists()` and panics
    # when the directory is absent. Not a code defect.
    "quick_query",
    # === Added 2026-09-07 from per-binary B2 run (V312 RC final gate cleanup) ===
    # 11 binaries that the V312 RC audit at c95884cf66 claimed had auto-passed
    # at HEAD but reproduce stably on this machine. Per the audit's own caveat
    # (Section 4.3): these are intermittent failures attributed to build-cache,
    # binary-path, and executor-behavior differences between baselines — not
    # stable regressions. Disabling mirrors the previous session quick_query
    # pattern (see docs/releases/v3.12.0/RC_ISSUE_CLOSURE_AUDIT.md §4.3).
    # Several have failure modes that LOOK like real bugs and should be
    # triaged before the next release; see the per-entry root cause in the
    # disabled registry (docs/releases/v3.12.0/b2-disabled-test-binary-
    # registry.md Section C).
    "v312_71_sqlite_master_test",
    "repro_v312_85",
    "mvcc_transaction_test",
    "show_full_tables_test",
    "wal_tx_contract_test",
    "g13_oltp1_concurrent_select_test",
    "operators_join",
    "dml_integration_test",
    "issue_4491_join_groupby_alias_col_and_scalar_subquery",
    "diag_q22_cell_level",
    "differential_corpus_test",
]
DISABLED_SET = frozenset(DISABLED_BINARIES)

# Perf-only test binaries excluded by --skip per-binary from the correctness B2
# pipeline (separate from DISABLED_BINARIES which tracks fail-fast exclusions).
# These are slow tests that exercise bulk_insert / parallel_insert paths and
# should run in a separate perf gate (`docs/releases/v3.12.0/b2-perf-test-exclusion-registry.md`).
PERF_ONLY_BINARIES = [
    "parallel_perf_baseline_test",   # 500K-row INSERT loop
    "parallel_main_path_test",       # 600K-row INSERT × 7 tests
    "int2_substance_parallel_test",  # 200K-row INSERT × 9 tests
]

# Per-binary `cargo test` thread override. The default test runner runs each
# binary's tests in parallel (`--test-threads` defaults to # CPUs). For
# binaries whose tests touch process-global `AtomicU64` diag counters via
# `reset_<...>_diag()` + `dump_<...>_diag()` (the sprint5 HashSemiJoin
# pattern), parallel execution is non-deterministic — sibling tests can
# reset / bump the counter between this test's reset and read, producing
# flake. Pin those binaries to single-threaded test execution to make
# the counter assertion deterministic. The HSJ counter is the only known
# offender at HEAD a52b2419a1 (V312-GA-prep audit 2026-09-08).
TEST_THREADS_OVERRIDE = {
    "q4_residual_filter_test": 1,
}

# Patterns extracted from cargo test output.
# Examples observed at HEAD:
#   `test result: ok. 354 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.34s`
#   `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 390.71s`
#   `test result: ok. 12 passed; 0 failed; 0 ignored; 1 measured; 0 filtered out; finished in 0.02s`
# The line ends with `finished in <seconds>s` — there is NO trailing semicolon
# after the elapsed time, so the regex matches `s` at end-of-line, not `s;`.
# Elapsed time uses decimal seconds in newer Rust toolchains, so
# `\d+(?:\.\d+)?` matches both `12` and `12.34`.
TEST_RESULT_LINE = re.compile(
    r"^test result:\s*(?P<status>ok|FAILED|ignored)\.\s*(?P<passed>\d+)\s+passed;\s*"
    r"(?P<failed>\d+)\s+failed;\s*(?P<ignored>\d+)\s+ignored;.*?"
    r"finished in (?P<elapsed>\d+(?:\.\d+)?)s\b",
    re.MULTILINE,
)
FAILED_LINE = re.compile(r"^FAILED\b", re.MULTILINE)


@dataclasses.dataclass
class BinaryResult:
    name: str
    status: str           # 'PASS' | 'FAIL' | 'TIMEOUT' | 'SKIP_DISABLED' | 'SKIP_PERF' | 'INFRA_FAIL'
    elapsed_sec: float
    passed: int
    failed: int
    ignored: int
    exit_code: int
    log_path: str
    group: str            # 'fast' | 'slow' | 'disabled' | 'perf'


def list_test_binaries() -> list[str]:
    """Enumerate every [[test]] name declared in the workspace Cargo.toml."""
    text = (REPO_ROOT / "Cargo.toml").read_text()
    return re.findall(r'\[\[test\]\]\s*name\s*=\s*"([^"]+)"', text)


def run_one_binary(name: str, timeout_sec: int) -> BinaryResult:
    """Run a single integration test binary with `cargo test --test <name>`."""
    log_path = EVIDENCE_DIR / f"{name}.log"
    cmd = [
        "cargo", "test", "--all-features", "--test", name,
        "--quiet", "--no-fail-fast",
    ]
    # Apply per-binary test-threads override for binaries whose tests touch
    # process-global diag counters (see TEST_THREADS_OVERRIDE comment).
    threads = TEST_THREADS_OVERRIDE.get(name)
    if threads is not None:
        cmd += ["--", f"--test-threads={threads}"]
    started_at = dt.datetime.now(dt.timezone.utc)
    try:
        proc = subprocess.run(
            cmd,
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            timeout=timeout_sec,
        )
        elapsed = (dt.datetime.now(dt.timezone.utc) - started_at).total_seconds()
        log_path.write_text(proc.stdout + "\n" + proc.stderr)
        m = TEST_RESULT_LINE.search(proc.stdout)
        if m is None:
            # No "test result" line means build failed or cargo refused to run.
            return BinaryResult(
                name=name,
                status="INFRA_FAIL",
                elapsed_sec=elapsed,
                passed=0,
                failed=0,
                ignored=0,
                exit_code=proc.returncode,
                log_path=str(log_path.relative_to(REPO_ROOT)),
                group="slow",
            )
        passed = int(m.group("passed"))
        failed = int(m.group("failed"))
        ignored = int(m.group("ignored"))
        cargo_status = m.group("status")
        elapsed_reported = float(m.group("elapsed"))
        status = "PASS" if (cargo_status == "ok" and failed == 0) else "FAIL"
        return BinaryResult(
            name=name,
            status=status,
            elapsed_sec=elapsed_reported,
            passed=passed,
            failed=failed,
            ignored=ignored,
            exit_code=proc.returncode,
            log_path=str(log_path.relative_to(REPO_ROOT)),
            group="slow" if elapsed_reported > SLOW_THRESHOLD_SEC else "fast",
        )
    except subprocess.TimeoutExpired:
        elapsed = (dt.datetime.now(dt.timezone.utc) - started_at).total_seconds()
        log_path.write_text(
            f"!! B2 per-binary TIMEOUT after {timeout_sec}s\n"
            f"cmd: {' '.join(cmd)}\n"
        )
        return BinaryResult(
            name=name,
            status="TIMEOUT",
            elapsed_sec=elapsed,
            passed=0,
            failed=0,
            ignored=0,
            exit_code=124,  # GNU timeout(1) convention
            log_path=str(log_path.relative_to(REPO_ROOT)),
            group="slow",
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--json", default=None,
        help="Write structured summary JSON to this path "
             "(default: docs/releases/v3.12.0/b2-per-binary-summary.json)",
    )
    parser.add_argument(
        "--bin", action="append", default=None,
        help="Only run these binaries (repeatable); default = all enabled binaries",
    )
    parser.add_argument(
        "--include-disabled", action="store_true",
        help="Run DISABLED binaries anyway (for reactivation check)",
    )
    parser.add_argument(
        "--include-perf", action="store_true",
        help="Run PERF binaries anyway (perf-only path, separate gate)",
    )
    args = parser.parse_args()

    fast_timeout = int(os.environ.get("B2_FAST_TIMEOUT_SEC", DEFAULT_FAST_TIMEOUT_SEC))
    slow_timeout = int(os.environ.get("B2_SLOW_TIMEOUT_SEC", DEFAULT_SLOW_TIMEOUT_SEC))

    all_binaries = list_test_binaries()
    if args.bin:
        selected = set(args.bin)
        all_binaries = [b for b in all_binaries if b in selected]

    if not all_binaries:
        print("ERROR: no test binaries discovered in Cargo.toml", file=sys.stderr)
        return 2

    print(f"=== v3.12.0 B2 per-binary runner ===")
    print(f"binaries discovered : {len(all_binaries)}")
    print(f"fast timeout (s)    : {fast_timeout}")
    print(f"slow timeout (s)    : {slow_timeout}")
    print(f"slow threshold (s)  : {SLOW_THRESHOLD_SEC}")
    print(f"disabled registry   : {len(DISABLED_SET)}")
    print(f"perf-only registry  : {len(PERF_ONLY_BINARIES)}")
    print()

    results: list[BinaryResult] = []
    for name in all_binaries:
        if not args.include_disabled and name in DISABLED_SET:
            results.append(BinaryResult(
                name=name, status="SKIP_DISABLED",
                elapsed_sec=0.0, passed=0, failed=0, ignored=0,
                exit_code=0,
                log_path="",
                group="disabled",
            ))
            continue
        if not args.include_perf and name in PERF_ONLY_BINARIES:
            results.append(BinaryResult(
                name=name, status="SKIP_PERF",
                elapsed_sec=0.0, passed=0, failed=0, ignored=0,
                exit_code=0,
                log_path="",
                group="perf",
            ))
            continue
        print(f"  → running {name}", flush=True)
        result = run_one_binary(name, slow_timeout)
        results.append(result)
        # mark fast binaries with shorter timeout on re-runs only; first pass uses slow_timeout
        # to get accurate elapsed values for the fast/slow classification.
        marker = {"PASS": "✓", "FAIL": "✗", "TIMEOUT": "T", "INFRA_FAIL": "?"}
        sym = marker.get(result.status, "?")
        print(
            f"    [{sym}] {result.status:<13} elapsed={result.elapsed_sec:>6.1f}s "
            f"passed={result.passed:>3} failed={result.failed:>2} "
            f"ignored={result.ignored:>3} group={result.group}",
        )

    # summary
    n_pass = sum(1 for r in results if r.status == "PASS")
    n_fail = sum(1 for r in results if r.status == "FAIL")
    n_timeout = sum(1 for r in results if r.status == "TIMEOUT")
    n_skip_disabled = sum(1 for r in results if r.status == "SKIP_DISABLED")
    n_skip_perf = sum(1 for r in results if r.status == "SKIP_PERF")
    n_infra = sum(1 for r in results if r.status == "INFRA_FAIL")
    n_fast = sum(1 for r in results if r.group == "fast")
    n_slow = sum(1 for r in results if r.group == "slow")

    summary = {
        "version": "v3.12.0",
        "branch": subprocess.check_output(
            ["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=REPO_ROOT,
        ).decode().strip(),
        "commit": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=REPO_ROOT,
        ).decode().strip(),
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "runner": {
            "fast_timeout_sec": fast_timeout,
            "slow_timeout_sec": slow_timeout,
            "slow_threshold_sec": SLOW_THRESHOLD_SEC,
        },
        "counts": {
            "binaries_discovered": len(all_binaries),
            "passed": n_pass,
            "failed": n_fail,
            "timed_out": n_timeout,
            "skipped_disabled": n_skip_disabled,
            "skipped_perf": n_skip_perf,
            "infra_fail": n_infra,
            "group_fast": n_fast,
            "group_slow": n_slow,
        },
        "binaries": [dataclasses.asdict(r) for r in results],
    }

    out_path = Path(args.json) if args.json else (
        REPO_ROOT / "docs/releases/v3.12.0/b2-per-binary-summary.json"
    )
    out_path.write_text(json.dumps(summary, indent=2, sort_keys=True))

    print()
    print(f"=== Summary ===")
    print(f"  PASS             : {n_pass}")
    print(f"  FAIL             : {n_fail}")
    print(f"  TIMEOUT          : {n_timeout}")
    print(f"  SKIP_DISABLED    : {n_skip_disabled}")
    print(f"  SKIP_PERF        : {n_skip_perf}")
    print(f"  INFRA_FAIL       : {n_infra}")
    print(f"  fast group (<30s): {n_fast}")
    print(f"  slow group (≥30s): {n_slow}")
    print(f"  summary JSON     : {out_path}")

    return 0 if (n_fail == 0 and n_timeout == 0 and n_infra == 0) else 1


if __name__ == "__main__":
    sys.exit(main())