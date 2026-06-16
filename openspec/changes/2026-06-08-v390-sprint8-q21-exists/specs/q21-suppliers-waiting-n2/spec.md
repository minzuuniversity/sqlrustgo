# Capability: q21-suppliers-waiting-hash-semi-join

## ADDED Requirements (target state, deferred to Sprint 8)

### Requirement: TPC-H Q21 (Suppliers Who Kept Orders Waiting) in-process < 5s at SF=0.1

The executor MUST, when running Q21 against an SF=0.1 fixture,
complete the 4-way JOIN with two correlated EXISTS subqueries
in < 5 seconds and return ~100 rows matching PostgreSQL.

#### Scenario: Q21 in-process < 5s at SF=0.1

- **GIVEN** Sprint 6 SF=0.1 fixture is loaded
- **WHEN** the executor runs Q21
- **THEN** wall-time < 5s
- **AND** ~100 rows matching PG row-for-row

#### Scenario: Q21 in-process < 60s at SF=1.0

- **GIVEN** SF=1.0 fixture
- **WHEN** the executor runs Q21
- **THEN** wall-time < 60s

#### Scenario: Q21 no regression on other correlated subquery queries

- **WHEN** the executor runs Q3, Q4, Q8, Q10, Q13, Q15, Q16
- **THEN** cell-level results match pre-hash-semi-join baseline

## Deferred Implementation (Sprint 8 backlog)

- [ ] `crates/executor/src/join/semi_join.rs` — new file, ~300 LOC
- [ ] Hash-semi-join for EXISTS subqueries
- [ ] Hash-anti-semi-join for NOT EXISTS subqueries
- [ ] Q21 cell-level regression test
- [ ] Q21 perf bench
- [ ] Estimated 5-10h engineering
