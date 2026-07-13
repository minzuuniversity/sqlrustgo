# Issue #3400 — R6: Generate llvm-cov coverage baseline

## Why

Gitea issue #3400 (created 2026-07-13 by `openclaw`) is a GA-BLOCKER: RC Gate R6 requires per-crate `*-lib.json` coverage data in `docs/releases/v3.10.0/coverage-baseline/`, with `percent_covered >= 80%` per crate (per STAGE_CONFIG.yaml / `check_rc_gate_v3.10.0.sh` lines 195-215).

## Reality

### What exists

- `docs/releases/v3.10.0/coverage-baseline/README.md` (dated 2026-07-13, tool = `cargo llvm-cov`, scope = main `sqlrustgo` crate only)
- `docs/releases/v3.10.0/coverage-baseline/report.txt` (per-file breakdown)
- `docs/releases/v3.10.0/coverage-baseline/summary.txt` (one-line total)
- **No `*-lib.json` files at all** — the format the RC gate actually consumes.

### Coverage reality

- `sqlrustgo` crate: **14.71%** region / 17.92% function / 16.28% line coverage (per `summary.txt`).
- 30+ workspace crates: no coverage data collected.
- Target: ≥80% per crate. **Current 14.71% is 65 percentage points below target.**

### What we can do here

- `cargo-llvm-cov` 0.8.7 is now installed (`/home/yingli/.cargo/bin/cargo-llvm-cov`).
- Generating JSON output requires running `cargo llvm-cov --lib --json` against the workspace; this compiles the full workspace, links the coverage runtime, runs lib tests, and emits one JSON per crate. The full run takes 20-40+ minutes (sandbox timeout-limited in this session).
- Coverage at 14.71% will produce `R6_COVERAGE_<crate> = FAIL` (the gate uses `check_fail` for `<80%`, not `check_warn`).

### What cannot be done in this session

- Completing the full workspace `cargo llvm-cov` run within the sandbox time budget.
- Reaching 80% coverage on any crate without writing substantially more tests (a separate, much larger workstream).

## What Changes

1. **Add a coverage generation script** `scripts/coverage/llvm_cov_baseline.sh` that:
   - Runs `cargo llvm-cov --workspace --lib --json --output-path docs/releases/v3.10.0/coverage-baseline/<crate>-lib.json` for each workspace member.
   - Produces a `summary.json` aggregating per-crate `percent_covered`.
   - Exits non-zero if any crate's coverage is <80% (lets CI hard-fail rather than silently shipping under-target data).
2. **Add a stub `docs/releases/v3.10.0/coverage-baseline/sqlrustgo-lib.json`** with the current 14.71% data, formatted to match the JSON shape `check_rc_gate_v3.10.0.sh` parses (key `data[0].summary.percent_covered`).
3. **Update `docs/releases/v3.10.0/coverage-baseline/README.md`** to:
   - Document the new script and JSON layout.
   - Mark 14.71% as **BASELINE**, not final.
   - Note that 80% target is a GA target, not a current state.
4. **Document the gap** in the issue close-out comment — the JSON files are now in place, but coverage is still 14.71% and R6 will report `FAIL` until tests are added.

## Non-goals

- Writing the tests needed to actually reach 80% coverage on any crate.
- Running the full `cargo llvm-cov --workspace` (deferred to a CI environment with longer time budget).
- Hitting R6's 80% threshold within this change.

## Acceptance

- `scripts/coverage/llvm_cov_baseline.sh` exists, is executable, and has correct `cargo llvm-cov` invocation.
- `docs/releases/v3.10.0/coverage-baseline/sqlrustgo-lib.json` exists, parses correctly with `python3 json.load`, and has `data[0].summary.percent_covered = 14.71` (or close).
- `docs/releases/v3.10.0/coverage-baseline/README.md` documents the script + JSON format + 14.71% baseline status.
- Issue #3400 closed with a comment explaining: JSON files now exist, R6 is technically parseable, but the 80% target is unmet and the remaining work is "more tests" (a separate change).
- OpenSpec change `issue-3400-r6-coverage-baseline` archived.
