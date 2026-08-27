# Proposal: Add --fast-path flag to scripts/gate/check_ga_v3.12.0.sh

## Why

Issue #4498 (GA-1 aggregator) and the follow-up #4536 observe a
design-vs-implementation inconsistency in
`scripts/gate/check_ga_v3.12.0.sh`:

- The script's header (lines 13-16) states:
  > "This script does NOT re-run heavy tests. It verifies that
  > each per-stage gate script exists, runs in fast path, and
  > reports PASS. Heavy verification ... is delegated to CI."

- But the `run_beta_gate()` function (lines 99-125) actually
  invokes `bash "$beta_script"` unconditionally — which on a
  dev laptop blocks the aggregator for >3 min on the
  `cargo build --all-features` + `cargo clippy` + `cargo fmt`
  cascade inside `check_beta_v3.12.0.sh`.

The aggregator is intended for both local-dev smoke (must be
<30s) and CI heavy-runs (full BETA execution). Adding an explicit
`--fast-path` flag reconciles the header claim with the actual
behavior.

## What changes

1. **scripts/gate/check_ga_v3.12.0.sh**:
   - Add CLI arg parser that accepts `--fast-path` and `--full`
     (default: `--fast-path` for local-dev ergonomics, `--full`
     opt-in for CI parity with the previous behavior).
   - When `--fast-path`: skip `run_beta_gate()` heavy BETA
     execution; instead perform a fast-path BETA check (script
     existence + `bash -n` syntax check), analogous to the
     existing RC / GA fast-path behavior at lines 162-170.
   - When `--full` (legacy): run the heavy `bash $beta_script`
     path. Keep this as the CI default.
   - Update header doc to make the two modes explicit.
   - Add `--help` output.

2. **Default behavior decision**:
   - The issue body states "默认行为保持向后兼容（仍跑完整
     stages）" — backward-compatible default.
   - Default = `--full` (preserves CI behavior). Local dev
     explicitly opts in via `--fast-path`.
   - This matches the issue's recommendation #3: "CI 默认走
     完整 stages，本地开发走 fast-path".

## Impact

- **#4498** (GA-1 aggregator): unblocked — fast-path now matches
  the header claim.
- **#4536** (this issue): closed once the flag is implemented
  and a 30-second smoke run on dev proves the fast-path mode
  actually skips the heavy BETA call.
- **#4497** (umbrella): one less governance UX defect.

## Non-goals

- Refactoring `check_beta_v3.12.0.sh` itself — its heavy
  `cargo build / clippy / fmt` cascade stays; this change
  only gives the aggregator a way to **skip** it locally.
- Adding new fast-path checks for RC / GA stages — they
  already are fast-path (per lines 162-170 and 226-247).

## Risk

- **Backwards compat**: existing CI invocations that pass no flag
  must continue to run the full BETA gate. Mitigation: default
  behavior = `--full`.
- **Local dev ergonomics**: users may forget `--fast-path` and
  see the same >3 min block. Mitigation: add a startup banner
  when the heavy BETA path is about to run, with a hint to use
  `--fast-path`.

## Stage gate

- Current: **RC (2026-08-26)**.
- This change is a **governance script UX improvement**. RC
  stage permits governance / infra / docs updates (the "受
  保护目录" list does not include `scripts/gate/`).
- No production code, no API change, no public protocol
  change. Risk: trivial.

## Provenance (ADR-014 5 evidence fields)

| 字段 | 值 |
|------|---|
| source_agent | claude-sonnet (Claude Code) |
| source_run | issue-4536-fast-path-ga-gate-20260827 |
| timestamp | 2026-08-27T22:15:00+08:00 |
| evidence_hash | local-git:`14f638d09` (post-merge of PR #4524 docs-unify) |
| conflict_resolution | N/A — single AI scope |

Refs: #4498 (GA-1 aggregator), #4536 (this issue), #4497
(umbrella), #4387 (V312-59-D prior cycle).