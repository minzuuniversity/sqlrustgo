# Architecture Decision Records (ADR) Index

> **Status**: ACTIVE
> **Last update**: 2026-07-01 (added a/b disambiguation to 4 duplicate ADR pairs; added ADR-014 row)
> **Maintainer**: Hermes Agent

This is the index for Architecture Decision Records (ADR) under
`docs/governance/adr/`. The root governance index is at
[`../INDEX.md`](../INDEX.md).

## How to use this index

ADR files document **why** a major architectural decision was made,
not just **what** was decided. Read the linked ADR before challenging
or changing any of the items below — the historical context and
rejected alternatives are usually recorded.

When creating a new ADR, append a row to the table below and
follow the file naming convention `ADR-NNN-<topic>.md`.

## Status legend

- **ACCEPTED** — Decision is in force.
- **PROPOSED** — Decision is under review, not yet binding.
- **SUPERSEDED** — Replaced by a later ADR (see "Supersedes" field).
- **DEFERRED** — Decision postponed to a future version.

## Active ADRs

| # | Title | Status | Topic | File |
|---|-------|--------|-------|------|
| ADR-001 | Truthfulness Framework | ACCEPTED | Test claim transparency foundation | [ADR-001-truthfulness-framework.md](ADR-001-truthfulness-framework.md) |
| ADR-002 | Claim Registry | ACCEPTED | Centralized list of "we said X" claims | [ADR-002-claim-registry.md](ADR-002-claim-registry.md) |
| ADR-003 | Decision Registry | ACCEPTED | ADR structure (this file) | [ADR-003-decision-registry.md](ADR-003-decision-registry.md) |
| ADR-004 | Negative Evidence | ACCEPTED | Documenting what did NOT work | [ADR-004-negative-evidence.md](ADR-004-negative-evidence.md) |
| ADR-005 | Legacy Gate Retirement | ACCEPTED | Gating policy for retired gates | [ADR-005-legacy-gate-retirement.md](ADR-005-legacy-gate-retirement.md) |
| **ADR-006a** | **Meta-Governance Framework (P11-P15)** [v3.9.0] | ACCEPTED | Gate self-verification, DRIFT handling, oracle requirement | [ADR-006-meta-governance.md](ADR-006-meta-governance.md) |
| **ADR-006b** | TX+WAL Contract Test Deferral [v3.8.0] | ACCEPTED | Why TX/WAL contract tests are deferred from v3.8.0 | [ADR-006-tx-wal-contract-deferral.md](ADR-006-tx-wal-contract-deferral.md) |
| **ADR-007a** | **5-PR Truthfulness Recovery Sequence (2026-06-17)** [v3.9.0] | ACCEPTED | 5-PR sequence to recover v3.9.0 test-claim truthfulness | [ADR-007-truthfulness-recovery-sequence.md](ADR-007-truthfulness-recovery-sequence.md) |
| **ADR-007b** | WAL Architecture Clarification [v3.8.0] | ACCEPTED | WAL buffering vs direct file storage | [ADR-007-wal-architecture-clarification.md](ADR-007-wal-architecture-clarification.md) |
| **ADR-008** | **Test Claim Transparency + No-Ignore Gate Tests Policy** | ACCEPTED | The P16 0-`#[ignore]` on gate tests policy | [ADR-008-test-claim-transparency.md](ADR-008-test-claim-transparency.md) |
| ADR-009 | G-01 Validation Chain Enforcement | ACCEPTED | Meta-gate G-01 enforcement | [ADR-009-g01-validation-chain-enforcement.md](ADR-009-g01-validation-chain-enforcement.md) |
| **ADR-010a** | **Cross-Version Debt Governance (G-02 follow-up)** [v3.8.0] | ACCEPTED | G-02 cross-version debt lifecycle | [ADR-010-cross-version-debt-governance.md](ADR-010-cross-version-debt-governance.md) |
| **ADR-010b** | Ghost PR Resolution — F-07~F-15 Formal Deferral [v3.8.0] | ACCEPTED | Why ghost PRs F-07..F-15 are formally deferred | [ADR-010-ghost-pr-resolution.md](ADR-010-ghost-pr-resolution.md) |
| **ADR-011a** | **Cross-Version Debt State Machine** [v3.9.0] | ACCEPTED | Debt state lifecycle (open→tracked→resolved) | [ADR-011-debt-state-machine.md](ADR-011-debt-state-machine.md) |
| **ADR-011b** | v3.8.0+1 TX+WAL Repair Strategy [v3.8.0+1] | ACCEPTED | The v3.8.0+1 release repair plan | [ADR-011-v3.8.0-1-tx-wal-repair.md](ADR-011-v3.8.0-1-tx-wal-repair.md) |
| ADR-012 | SQLRustGo vs GMP-Platform Scope Boundary | ACCEPTED | Project boundary clarification | [ADR-012-sqlrustgo-vs-gmp-platform-scope.md](ADR-012-sqlrustgo-vs-gmp-platform-scope.md) |
| **ADR-013** | **v3.10 Wired-Soak DDL + Wire Protocol 修复 RFC** | PROPOSED | v3.10 milestone plan (closes Issue #3302, 4-PR plan) | [ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md](ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md) |
| **ADR-008x** | **ADR-008 §Policy 2 Exception: v3.11.0 G4 TPC-H SF=1 Wire Test Deferral** | PROPOSED | First use of ADR-008 exception mechanism; expires 2026-09-01 | [ADR-008-exception-v311-tpch-sf1.md](ADR-008-exception-v311-tpch-sf1.md) |
| **ADR-008y** | **ADR-008 §Policy 2 Exception: V312-58 Q17 SF=1 Perf Benchmark Deferral** | PROPOSED | Second use of ADR-008 exception; expires 2026-10-31; GA-5 cell-diff gate carries canonical PASS artifact | [ADR-008-exception-v312-58-q17-sf1.md](ADR-008-exception-v312-58-q17-sf1.md) |
| **ADR-014** | Multi-AI Coordination | ACCEPTED | Z6G4 + macmini + future AI agent protocol (5 evidence fields, conflict resolution) | [ADR-014-multi-ai-coordination.md](ADR-014-multi-ai-coordination.md) |

## Exception ADRs (ADR-008 §Policy 2)

ADR-008 §Policy 2 allows gate tests to be temporarily ignored only via a documented
ADR amendment with explicit deadline, owner, and success criteria. The two existing
exceptions are:

- **ADR-008x** (2026-08-09, expires 2026-09-01) — v3.11.0 G4 TPC-H SF=1 wire test deferral
- **ADR-008y** (2026-08-31, expires 2026-10-31) — V312-58 Q17 SF=1 perf benchmark deferral

Exception suffix convention: `ADR-NNNx` (e.g. `ADR-008x`, `ADR-008y`). The underlying
policy ADR retains its base number; each exception is a separate file with its own
proposal status and deadline.

## Numbering notes

The same ADR number is sometimes used for two different decisions.
To disambiguate, each duplicate is suffixed with `a` (the v3.9.0-era
decision, generally newer/larger content) or `b` (the v3.8.0-era
decision, generally the historical original). Examples:

- **ADR-006a** = Meta-Governance Framework (P11-P15) [v3.9.0, 2026-06-13]
- **ADR-006b** = TX+WAL Contract Test Deferral [v3.8.0, 2026-06-03]
- **ADR-007a** = 5-PR Truthfulness Recovery Sequence [v3.9.0, 2026-06-17]
- **ADR-007b** = WAL Architecture Clarification [v3.8.0, 2026-06-03]
- **ADR-010a** = Cross-Version Debt Governance [v3.8.0, 2026-06-03]
- **ADR-010b** = Ghost PR Resolution [v3.8.0, 2026-06-03]
- **ADR-011a** = Cross-Version Debt State Machine [v3.9.0, 2026-06-05]
- **ADR-011b** = v3.8.0+1 TX+WAL Repair Strategy [v3.8.0+1]

**Note**: Suffixes are an in-index label only — the actual files keep
their original `ADR-NNN-<topic>.md` filename (no breakage of existing
cross-references). When the next v3.10+ ADR is added, the next
available sequential number is used (currently 015+).

**Decision date**: 2026-07-01. ADR renumbering is **not** performed
because ~10+ cross-references in v3.8.0 specs and v3.9.0 RC/beta
release notes depend on the original 4 digit numbers and would
require coordinated updates across the doc tree. The `a`/`b`
suffix is a low-risk disambiguation that preserves all existing
links while making the duplication visible at a glance.

The original "Numbering notes" disclaimer above (saying duplicates
"should be read independently") is preserved in spirit but augmented
with the a/b convention for clarity.
