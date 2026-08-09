## Context

V312-17 addresses test infrastructure debt preventing v3.12.0 RC gate execution:

**Current State:**
- `cargo test --no-run` fails with 15+ compilation errors
- 2 parser tests disabled with `#[ignore]` (API drift)
- 1 flaky WAL performance test (`test_wal_perf_throughput`)
- No canonical per-crate coverage measurement
- SEM-4 (coverage) still IN_PROGRESS per debt-registry.yaml

**Compilation Errors Identified:**
1. `ExecutionEngine` missing generics in tpch_test.rs
2. Unresolved imports: `vector_storage`, `vectorization`, `QueryCache`, `ConnectionPool`, `MvccEngine`
3. `Value::Date` variant missing in datetime_type_test.rs
4. Type mismatches in batch_insert_test.rs

**Constraints:**
- Must not break existing passing tests
- Must maintain backward compatibility for test APIs
- Coverage command must be reproducible across machines

## Goals / Non-Goals

**Goals:**
- Restore `cargo test --all-features --no-run` to green (all tests compile)
- Restore 2 disabled parser tests or formally quarantine with owner/expiry
- Fix `test_wal_perf_throughput` flakiness
- Document canonical coverage command: `cargo llvm-cov --workspace --tests --all-features`
- Generate per-crate coverage reports for RC gate

**Non-Goals:**
- Not fixing functional bugs exposed by restoring disabled tests (separate issues)
- Not achieving ≥80% coverage (deferred to V312-17 follow-up)
- Not refactoring core APIs (API drift fixes only)

## Decisions

### D1: Compilation Error Resolution Priority
**Decision:** Fix compilation errors before any other work - nothing else matters if tests don't compile.
**Rationale:** Blocking all test execution; 15+ errors indicate API surface changes.
**Alternatives:** Could skip broken tests and run subset - rejected per governance requirements.

### D2: Disabled Test Strategy
**Decision:** For each disabled test:
- If functionality still valid → restore by removing `#[ignore]`
- If functionality broken → quarantine with issue+owner+expiry (max 1 release)
- If no longer relevant → retire with evidence
**Rationale:** Blind restore could expose latent bugs; quarantine ensures accountability.

### D3: Flaky Test Fix
**Decision:** Remove timing assertion from `test_wal_perf_throughput`; measure once and report without assertion.
**Rationale:** Timing-based assertions inherently flaky across hardware/load conditions.
**Alternatives:** Add retry logic - rejected as masking root cause.

### D4: Coverage Command
**Decision:** Canonical command: `cargo llvm-cov --workspace --tests --all-features --open`
- Primary: `--tests` (runs integration tests)
- Fallback: `--lib` only if `--tests` produces no data
- Report: per-crate JSON format for RC gate R6
**Rationale:** Aligns with v3.10.0 R6 gate documented in `check_rc_gate_v3.10.0.sh`.

## Risks / Trade-offs

[Risk] Restoring disabled tests may expose latent bugs → **Mitigation**: Run tests before/after; if new failures, quarantine instead of revert

[Risk] API drift fixes may differ from original intent → **Mitigation**: Minimal changes; complex fixes defer to separate issue

[Risk] Coverage may still be below 80% after compilation fixes → **Mitigation**: Document current state; defer coverage improvement to follow-up issue with owner/expiry

## Open Questions

1. Should `Value::Date` be added to the types enum, or should datetime tests use a different approach?
2. Are the vector_storage/vectorization imports from deleted extension crates?
3. Who owns the follow-up issue for ≥80% coverage target?
