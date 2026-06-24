# Architecture Decision Records (ADR) Index

> **Status**: ACTIVE
> **Last update**: 2026-06-24
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
| **ADR-006** | **Meta-Governance Framework (P11-P15)** | ACCEPTED | Gate self-verification, DRIFT handling, oracle requirement | [ADR-006-meta-governance.md](ADR-006-meta-governance.md) |
| ADR-006 | TX+WAL Contract Test Deferral | ACCEPTED | Why TX/WAL contract tests are deferred from v3.8.0 | [ADR-006-tx-wal-contract-deferral.md](ADR-006-tx-wal-contract-deferral.md) |
| **ADR-007** | **5-PR Truthfulness Recovery Sequence (2026-06-17)** | ACCEPTED | 5-PR sequence to recover v3.9.0 test-claim truthfulness | [ADR-007-truthfulness-recovery-sequence.md](ADR-007-truthfulness-recovery-sequence.md) |
| ADR-007 | WAL Architecture Clarification | ACCEPTED | WAL buffering vs direct file storage | [ADR-007-wal-architecture-clarification.md](ADR-007-wal-architecture-clarification.md) |
| **ADR-008** | **Test Claim Transparency + No-Ignore Gate Tests Policy** | ACCEPTED | The P16 0-`#[ignore]` on gate tests policy | [ADR-008-test-claim-transparency.md](ADR-008-test-claim-transparency.md) |
| ADR-009 | G-01 Validation Chain Enforcement | ACCEPTED | Meta-gate G-01 enforcement | [ADR-009-g01-validation-chain-enforcement.md](ADR-009-g01-validation-chain-enforcement.md) |
| **ADR-010** | **Cross-Version Debt Governance (G-02 follow-up)** | ACCEPTED | G-02 cross-version debt lifecycle | [ADR-010-cross-version-debt-governance.md](ADR-010-cross-version-debt-governance.md) |
| ADR-010 | Ghost PR Resolution — F-07~F-15 Formal Deferral | ACCEPTED | Why ghost PRs F-07..F-15 are formally deferred | [ADR-010-ghost-pr-resolution.md](ADR-010-ghost-pr-resolution.md) |
| **ADR-011** | **Cross-Version Debt State Machine** | ACCEPTED | Debt state lifecycle (open→tracked→resolved) | [ADR-011-debt-state-machine.md](ADR-011-debt-state-machine.md) |
| ADR-011 | v3.8.0+1 TX+WAL Repair Strategy | ACCEPTED | The v3.8.0+1 release repair plan | [ADR-011-v3.8.0-1-tx-wal-repair.md](ADR-011-v3.8.0-1-tx-wal-repair.md) |
| ADR-012 | SQLRustGo vs GMP-Platform Scope Boundary | ACCEPTED | Project boundary clarification | [ADR-012-sqlrustgo-vs-gmp-platform-scope.md](ADR-012-sqlrustgo-vs-gmp-platform-scope.md) |
| **ADR-013** | **v3.10 Wired-Soak DDL + Wire Protocol 修复 RFC** | PROPOSED | v3.10 milestone plan (closes Issue #3302, 4-PR plan) | [ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md](ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md) |

## Numbering notes

The same ADR number is sometimes used for two different decisions
(e.g. ADR-006 has both "Meta-Governance" and "TX+WAL Contract
Deferral"; ADR-007 has both "Truthfulness Recovery" and "WAL
Architecture"). This happened because the ADR counter was reset
during the v3.8.0/v3.9.0 transition; the duplicates are **not
related decisions** and should be read independently. Future
ADRs use a new number for each distinct decision.

When a v3.10+ ADRs are added (e.g. ADR-014), the next available
number is used; this index will be updated to reflect.
