# ADR-006: TX+WAL Contract Test Deferral

## Status

**Accepted** — v3.8.0 GA (2026-06-03)

## Context

During v3.8.0 Beta Gate verification (2026-06-03), 31-case contract
test suite `tx_wal_contract_tests.rs` was evaluated. Result:

```
19 passed; 12 failed; 0 ignored
```

The 12 failures decompose into two independent root causes:

1. **TX-Lifecycle (4 tests)**: EEK v0 spec assumes `DML without active
   tx → Err`. Current implementation uses implicit autocommit.
2. **WAL Recovery (8 tests)**: Multi-tx ordering and partial-write
   semantics differ from spec assumptions.

Both gaps are **pre-existing** in v3.8.0 — not introduced by the
recent `feat(storage): FileStorage trigger persistence` (commit c8ee597ba)
or `test(storage): e2e WAL trigger recovery` (commit 96d52d4bf) work.

## Decision

**Defer the 12 contract gaps to post-v3.8.0 work**, with the following
rationale and conditions.

### Why Defer

1. **Stability over completeness** — v3.8.0 Beta Gate 14/14 PASS,
   plus the 8/8 newly added (e2e 3/3 + exp_g 5/5) recovery tests
   provide strong evidence that the current WAL/trigger integration
   works for the supported use cases. The 12 contract gaps target
   edge cases not exercised by any other test.

2. **High-blast-radius fixes** — Both gaps touch core subsystems:
   - TX-lifecycle change risks breaking existing autocommit callers
     (incl. the 19 passing contract tests and the 8/8 e2e tests)
   - WAL replay change risks breaking 5/5 `exp_g_wal_contracts_verified`
     tests that currently validate the simpler WAL contract
   Either fix requires multi-PR dedicated spike work, not a Beta
   Gate patch.

3. **Architectural decisions pending** — ISSUE-2742 (WAL Architecture
   Clarification) covers 4 architecture questions whose answers
   determine the correct WAL recovery semantics. Fixing Category B
   before ISSUE-2742 is resolved risks implementing the wrong design.

4. **Honest documentation > silent failure** — Deferring with a
   documented gap (ISSUE-2743) is more truthful than rushing a fix
   that may introduce regressions. The 19/31 passing tests establish
   a v3.8.0 contract baseline that future work can build on.

### Conditions for Deferral Acceptance

- [x] All 12 failures have a documented root cause in ISSUE-2743
- [x] All 19 passing tests are confirmed to remain green
- [x] No existing test regresses due to the deferral (verified by
      running e2e_trigger_wal_recovery + exp_g_wal_contracts_verified)
- [x] Reproduction recipe captured in ISSUE-2743
- [x] Post-v3.8.0 work items enumerated (spike tasks)

### What v3.8.0 Ships With

- 19/31 contract tests passing as v3.8.0 baseline
- 8/8 e2e + exp_g tests passing (3 + 5)
- 2 commits added to `local/v3.8.0-wal-integration`:
  - `c8ee597ba` feat(storage): FileStorage trigger persistence
  - `96d52d4bf` test(storage): e2e WAL trigger recovery
- `tests/tx_wal_contract_tests.rs` remains **untracked** in working tree
  (not committed to v3.8.0; signals "evaluation artifact, not v3.8.0 spec")

### What v3.8.0 Does NOT Ship With

- EEK v0 strict TX-lifecycle enforcement (deferred to EEK v1)
- Multi-tx WAL recovery ordering guarantee
- Partial-write idempotency guarantee

## Consequences

### Positive

- v3.8.0 GA proceeds on time with verified, working core
- Future contributors have a clear gap list (ISSUE-2743) to attack
- 19 passing contract tests provide a regression baseline for the 12
  pending fixes

### Negative

- v3.8.0 is officially incomplete w.r.t. the EEK v0 contract spec
- Users who rely on autocommit semantics cannot migrate to "strict
  DML-tx" mode in v3.8.0
- Partial-write recovery remains best-effort, not strict

### Risks

- **Risk R-1**: ISSUE-2743 may be forgotten after v3.8.0 GA.
  **Mitigation**: ISSUE-2743 references this ADR; both files remain
  in `docs/audit/issues/` and `docs/governance/adr/` per ADR-003
  decision registry rules.

- **Risk R-2**: The 19 passing tests may be removed/refactored in
  v3.8.0+1, losing the baseline.
  **Mitigation**: Tag the 19 tests as "v3.8.0 contract baseline"
  in a follow-up tracking issue. No tag added in this commit to
  avoid scope creep.

## References

- ISSUE-2743: TX+WAL Contract Test Gaps
- ISSUE-2742: WAL Architecture Clarification
- ISSUE-2740: Crash Recovery Correctness Unverified (related, RB-1 P0)
- ADR-001: Truthfulness Framework (Honest documentation over silent failure)
- ADR-003: Decision Registry (this ADR is registered)
- ADR-005: Legacy Gate Retirement (deferral semantics consistent)

## Revision History

- 2026-06-03: Accepted (李哥 / Hermes Agent working session)
