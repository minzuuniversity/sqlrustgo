# Capability: q8-national-market-share-hash-join

## ADDED Requirements (target state, deferred to Sprint 8)

### Requirement: TPC-H Q8 (National Market Share Query) in-process evaluation completes in < 5s at SF=0.1

The executor MUST, when running Q8 against an SF=0.1 fixture,
complete the 8-way JOIN with CASE WHEN aggregate in < 5 seconds
and return 2 rows matching PostgreSQL.

#### Scenario: Q8 in-process < 5s at SF=0.1

- **GIVEN** Sprint 6 SF=0.1 fixture is loaded
- **WHEN** the executor runs Q8
- **THEN** wall-time < 5s
- **AND** 2 rows matching PG row-for-row

#### Scenario: Q8 in-process < 60s at SF=1.0

- **GIVEN** SF=1.0 fixture
- **WHEN** the executor runs Q8
- **THEN** wall-time < 60s

#### Scenario: Q8 no regression on other JOINs

- **WHEN** the executor runs Q1-Q7, Q10-Q22
- **THEN** cell-level results match pre-hash-join baseline

## Deferred Implementation (Sprint 8 backlog)

- [ ] Hash-join for 8-way JOINs (builds on Q3 foundation)
- [ ] CASE WHEN short-circuit per row
- [ ] EXTRACT YEAR function support
- [ ] Q8 cell-level regression test
- [ ] Q8 perf bench
- [ ] Estimated 8-12h engineering
