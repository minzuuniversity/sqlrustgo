# v3.12.0 B2_INTEGRATION_TESTS — Perf-Only Test Binary Exclusion Registry

> **provenance**: generated_by=claude-code v3.12.0, generated_at=2026-08-24T08:05:00Z,
> source_run=fix/v4413-b2-per-binary-split @ f62c6dd14f,
> gate_runner=scripts/gate/run_b2_per_binary.py,
> gate_baseline=scripts/gate/check_beta_v3.12.0.sh B2_INTEGRATION_TESTS
> **policy**: Anti-Fabrication-Policy-v1.0 + ADR-008 (Test Claim Transparency)
> **scope**: Test binaries that are **correct but slow** (≥30s wall-time per binary
> on the HP Z6 G4 dev workstation). Distinct from
> [`b2-disabled-test-binary-registry.md`](b2-disabled-test-binary-registry.md)
> which tracks **fail-fast exclusions** (binaries with FAILED tests at HEAD).
> Perf-only binaries are excluded from the **correctness B2 gate** and re-routed
> to a separate perf gate (B2_PERF_BENCH — tracked in Issue #4413 acceptance
> criteria, sub-task 4).
> **expiry_policy**: All entries expire **2026-12-31** (extended window — perf
> refactors are larger scope than bug fixes). Reactivation path = refactor to
> `engine.bulk_insert_records()` (PR #4417 introduced the API) so total per-binary
> wall-time drops below 30s; then move the entry back to the B2 correctness gate.

## Why a separate registry?

`cargo test --test <binary>` cannot distinguish a "passing slow test" from a
"failing fast test" from exit-code alone — cargo just emits
`test result: ok. N passed; 0 failed` and the total elapsed time. When the
monolithic `cargo test --all-features --test '*'` invocation in
`scripts/gate/check_beta_v3.12.0.sh B2_INTEGRATION_TESTS` accumulates >30s per
binary, the total wall-time becomes unbounded (10+ min on small machines). Codex
feedback 2026-08-23T17:51:10Z required per-binary timeout + <30s/≥30s split
with **owner / reason / recovery condition / expiry version** for every
disabled or perf-only entry. The split into two files keeps the bug-tracking
registry short while giving perf-only entries a different expiry window.

## Owner and reactivation

Each entry has:
- **owner**: GitHub handle accountable for the perf refactor before expiry
- **reason**: why the binary is slow (specific `populate()` pattern or test loop)
- **recovery_condition**: precise change that would drop wall-time below 30s
- **expiry_version**: target version for reactivation (default v3.13.0)

Removing an entry requires (a) the recovery_condition being met (timed locally
on dev workstation + spec hardware), (b) verifying the binary runs in <30s,
(c) deleting the row from this file, and (d) updating `PERF_ONLY_BINARIES` in
`scripts/gate/run_b2_per_binary.py` (or removing the entry entirely if
`DISABLED_BINARIES` already excludes it).

## Registry entries (3 perf-only test binaries, captured 2026-08-24)

| # | Test binary | Slow reason | Owner | Recovery condition | Expiry version |
|---|---|---|---|---|---|
| 1 | `int2_substance_parallel_test` | `populate()` runs `for i in 0..(PARALLEL_MIN_ROWS / 10)` = 200,000 single-row INSERTs; binary has 9 tests × populate pattern → ~30 min/binary | openclaw | Replace `populate()` body with `engine.bulk_insert_records(table, records)` (PR #4417) or `INSERT INTO t SELECT ... FROM generate_series(0, N-1)`; verify per-binary wall-time <30s on dev workstation | v3.13.0 (2026-09-30 stage transition) |
| 2 | `parallel_main_path_test` | `populate()` runs `for i in 0..ROWS` (ROWS=600,000) single-row INSERTs; binary has 7 tests × populate → ~50 min/binary | openclaw | Replace `populate()` body with `engine.bulk_insert_records(table, records)` (PR #4417); verify per-binary wall-time <30s | v3.13.0 |
| 3 | `parallel_perf_baseline_test` | `populate()` runs `for i in 0..rows` (rows=500,000) single-row INSERTs; 4 tests × populate → ~40 min/binary | openclaw | Same as #2 — use `engine.bulk_insert_records()`; verify per-binary wall-time <30s | v3.13.0 |

## Total: 3 perf-only entries

These 3 binaries also appear in
[`b2-disabled-test-binary-registry.md`](b2-disabled-test-binary-registry.md) as
entries 18, 19, 27. They are listed here separately to make the
**correctness** vs **perf-only** distinction explicit:

- `b2-disabled-test-binary-registry.md` entry 18 → here entry 1
- `b2-disabled-test-binary-registry.md` entry 19 → here entry 2
- `b2-disabled-test-binary-registry.md` entry 27 → here entry 3

The duplication is intentional: the disabled registry tracks the **bug** (the
test is failing OR the binary is unsupportably slow); the perf exclusion
registry tracks the **path to reactivation** (perf refactor → bulk_insert API).

When the perf refactor closes one of these entries, remove it from BOTH files
and remove the binary name from `DISABLED_TESTS_LIST` in
`scripts/gate/check_beta_v3.12.0.sh` AND from `PERF_ONLY_BINARIES` in
`scripts/gate/run_b2_per_binary.py`.

## Runner integration

`scripts/gate/run_b2_per_binary.py` reads `PERF_ONLY_BINARIES` and emits
`status=SKIP_PERF` for each entry by default. To re-route the perf-only path
through the B2 gate (e.g. for the v3.13.0 perf refactor PR), pass
`--include-perf` to the runner. To verify reactivation locally:

```bash
# Test single binary with short timeout (fast iteration)
python3 scripts/gate/run_b2_per_binary.py --bin parallel_main_path_test --include-perf
# Confirm wall-time <30s before removing the entry
```

## Gate integration

The new B2_INTEGRATION_TESTS gate (after #4413 lands) becomes:

```bash
# In scripts/gate/check_beta_v3.12.0.sh (Issue #4413 acceptance)
check "B2_INTEGRATION_TESTS" "python3 scripts/gate/run_b2_per_binary.py --json /tmp/b2.json"
```

The runner produces
[`b2-per-binary-summary.json`](b2-per-binary-summary.json) which contains the
fast/slow split counts, per-binary elapsed time, and the SKIP_DISABLED /
SKIP_PERF tallies. Exit code 0 = gate PASS, exit code 1 = gate FAIL.

## Acceptance criteria (Issue #4413, sub-task 4)

- [x] Document file exists: `docs/releases/v3.12.0/b2-perf-test-exclusion-registry.md`
- [x] Every entry has owner / reason / recovery_condition / expiry_version
- [x] `PERF_ONLY_BINARIES` in `scripts/gate/run_b2_per_binary.py` mirrors the registry
- [x] Runner emits `SKIP_PERF` status for each entry by default
- [x] `--include-perf` flag re-enables the perf-only path (used for refactor verification)

## Anti-pattern gates (Codex feedback 2026-08-23)

1. **Do not** claim B2 PASS without running the new per-binary runner end-to-end.
   The runner's exit code is the gate; do not extrapolate from partial runs.
2. **Do not** collapse the perf-exclusion registry back into the disabled
   registry — perf refactors have different ownership and a longer expiry
   window. Keep the split.
3. **Do not** skip a perf-only binary in the perf gate without documenting
   the reason (the four columns: owner / reason / recovery / expiry).
4. **Do not** reactivate a perf-only binary by adding it to `DISABLED_BINARIES`
   instead — that hides a slow binary rather than fixing it.

## Reactivation workflow (closed by v3.13.0)

1. **Apply the recovery condition** (e.g. replace `populate()` with
   `engine.bulk_insert_records()` calls)
2. **Local verification**: `python3 scripts/gate/run_b2_per_binary.py --bin
   <binary> --include-perf` → wall-time <30s, status=PASS
3. **Update this registry**: delete the row for that binary
4. **Update the disabled registry**: also delete the duplicate row in
   `b2-disabled-test-binary-registry.md`
5. **Update the gate**: remove the binary name from `DISABLED_TESTS_LIST` in
   `scripts/gate/check_beta_v3.12.0.sh` AND from `PERF_ONLY_BINARIES` in
   `scripts/gate/run_b2_per_binary.py`
6. **Run B2_INTEGRATION_TESTS** to confirm 0 failures
7. **Commit + PR** with reference to the closing issue (#4417 + refactor)

## Provenance and audit trail

- **Source**: `cargo test --all-features --test <perf-binary>` timing at HEAD
  `f62c6dd14f` (develop/v3.12.0), measured on HP Z6 G4 dev workstation
- **Captured**: 2026-08-24T08:05:00Z
- **Evidence**: per-binary elapsed time emitted by
  `scripts/gate/run_b2_per_binary.py` (group: `slow`)
- **Related**: [`b2-disabled-test-binary-registry.md`](b2-disabled-test-binary-registry.md)
  (entries 18, 19, 27 are the duplicates of the entries above)

## Follow-up: Issue #4413 acceptance criteria mapping

| #4413 criterion | Met by |
|---|---|
| (A) per-binary invocation `cargo test --test <bin> --all-features` | `run_b2_per_binary.py::run_one_binary` |
| (B) per-binary timeout | `run_b2_per_binary.py::run_one_binary` (subprocess.run timeout) |
| (C) <30s/>30s split | `run_b2_per_binary.py::BinaryResult.group` |
| (D) per-binary log | `evidence/b2_per_binary/<bin>.log` |
| (E) disabled list enforced | `run_b2_per_binary.py::DISABLED_BINARIES` |
| (F) perf-exclusion registry | **This file** |
| (G) owner / reason / recovery / expiry for every excluded entry | **This file** + `b2-disabled-test-binary-registry.md` |

All 7 criteria met by PR `fix/v4413-b2-per-binary-split` (current branch).