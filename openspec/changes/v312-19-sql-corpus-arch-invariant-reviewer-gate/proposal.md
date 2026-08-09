## Why

V312-19 / ISSUE #3906 promotes three items still listed as TBD in the v3.11.0 `RELEASE_GATE_CHECKLIST.md` to v3.12 RC/GA blocking gates. Per the v3.11.0 comprehensive assessment, the current release-checklist has a `test_sql_corpus.sh all targets: TBD` row, the R2.1-R2.8 architecture-invariant scripts (`scripts/gate/check_arch_invariants.sh` and siblings) are not yet emitted as a single reviewable artifact, and the dual-reviewer sign-off template does not exist. Without these three, the v3.12 RC cannot truthfully claim a release-signoff gate, and a single reviewer (or a single run) can rubber-stamp PASS claims. This is also the gating parent for #3900 (wire) and #3908 (compat) — both feed into the same `test_sql_corpus.sh` and reviewer template.

## What Changes

- **`test_sql_corpus.sh` all-targets report**: extend the existing script (located at `scripts/gate/test_sql_corpus.sh`) to enumerate every target crate that registers a corpus (parser fixtures, SQLLogicTest, TPC-H queries, wire-protocol corpus, MySQL-compat corpus) and emit a single markdown table to `docs/releases/v3.12.0/evidence/sql_corpus/ALL_TARGETS_REPORT.md` with columns: target | cases | pass | fail | skipped | evidence_hash | timestamp | source_run.
- **R2.1-R2.8 invariant scripts unified artifact**: wrap the existing `check_arch2_no_bypass.sh`, `check_arch3_no_bypass.sh`, `check_arch_invariants.sh`, `check_arch_sem_debt.sh` and the 5 missing variants (R2.5-R2.8) in a driver `scripts/gate/check_r2_invariants.sh` that emits `docs/releases/v3.12.0/evidence/arch_invariants/R2_INVARIANTS_REPORT.md` containing the per-rule pass/fail, the command, the exit code, and a SHA256 of the captured stdout.
- **Reviewer sign-off template** at `docs/governance/REVIEWER_SIGNOFF_TEMPLATE.md`: a markdown form with fields for issue, branch, commit, gate report path, evidence hash, reviewer A (login + signature block), reviewer B (login + signature block), each row requiring `command output | timestamp | source_agent | source_run | evidence_hash | output location`.
- **Gate integration** in `scripts/gate/check_rc_ga_gate.sh` (or new `check_v312_19_release_gates.sh`): refuse RC sign-off unless all three artifacts above are present and within 7 days of `git rev-parse HEAD`.
- **CI helper** `scripts/gate/assert_reviewer_signoff.sh`: structural check that the sign-off file is parseable, references the right commit SHA, and lists two distinct reviewers.

## Capabilities

### New Capabilities

- `sql-corpus-all-targets-report`: enumerated, machine-readable report for every SQL corpus target the v3.12 RC ships.
- `arch-invariant-r2-unified-report`: a single artifact bundling the R2.1-R2.8 architectural invariant checks with exit codes and SHA256 of each script's output.
- `reviewer-signoff-template`: a governance-enforced two-reviewer sign-off form tied to commit SHA, gate report, and evidence hash.

### Modified Capabilities

- `gate-rc-ga-checklist`: require the three new artifacts as blocking pre-conditions for RC → GA promotion.

## Impact

- **New**: `docs/governance/REVIEWER_SIGNOFF_TEMPLATE.md`.
- **New**: `scripts/gate/check_r2_invariants.sh`, `scripts/gate/check_v312_19_release_gates.sh`, `scripts/gate/assert_reviewer_signoff.sh`.
- **Modified**: `scripts/gate/test_sql_corpus.sh` to emit the all-targets report (preserving existing stdout behavior).
- **Modified**: `scripts/gate/check_rc_ga_gate.sh` to call the new gate.
- **New (generated)**: `docs/releases/v3.12.0/evidence/sql_corpus/ALL_TARGETS_REPORT.md`, `docs/releases/v3.12.0/evidence/arch_invariants/R2_INVARIANTS_REPORT.md`.
- **No new crate deps**. CI runs `bash scripts/gate/check_v312_19_release_gates.sh` and fails the RC promotion if any artifact is missing.
