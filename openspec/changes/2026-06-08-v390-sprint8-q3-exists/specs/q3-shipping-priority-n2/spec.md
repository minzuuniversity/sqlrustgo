# Capability: q3-shipping-priority-hash-join

## ADDED Requirements (target state, deferred to Sprint 8)

### Requirement: TPC-H Q3 (Shipping Priority Query) in-process evaluation must complete in < 1s at SF=0.1

The executor MUST, when running Q3 against an SF=0.1 fixture
(60K lineitem, 20K orders, 15K customers), complete the
3-way JOIN (customer × orders × lineitem) with the WHERE filter
and GROUP BY aggregate in < 1 second wall-time, and return 10
rows matching PostgreSQL.

#### Scenario: Q3 in-process eval < 1s at SF=0.1

- **GIVEN** the Sprint 6 SF=0.1 fixture is loaded into in-process
      storage
- **WHEN** the executor runs the Q3 SQL form
- **THEN** the wall-time is < 1s (target: 0.1s for the JOIN +
      0.05s for the aggregate + LIMIT)
- **AND** the result has 10 rows matching PG row-for-row

#### Scenario: Q3 in-process eval < 30s at SF=1.0

- **GIVEN** an SF=1.0 fixture (6M lineitem, 1.5M orders, 1.2M
      customers) is loaded
- **WHEN** the executor runs Q3
- **THEN** the wall-time is < 30s

#### Scenario: Q3 hash-join does not regress other JOINs

- **WHEN** the executor runs Q1, Q2, Q4, Q5, Q6, Q7, Q10, Q12,
      Q13, Q14, Q15, Q16, Q17, Q19, Q20, Q22
- **THEN** the cell-level results match the pre-hash-join baseline
      (no regression)

## Deferred Implementation (Sprint 8 backlog)

- [ ] `crates/executor/src/join/hash_join.rs` — new file, ~300 LOC
- [ ] `src/engine_select.rs` — route 3+ way JOINs to hash-join
      when the WHERE has ≥ 2 equi-join conditions
- [ ] `tests/sprint8_q3_regression_test.rs` — 3 tests
      (SF=0.1 cell-match, SF=0.1 timing < 1s, no-regression on
      other JOINs)
- [ ] `benches/sprint8_q3_bench.rs` — 4 sizes (SF=0.01, 0.1,
      1.0, 10.0)
- [ ] Estimated effort: 5-15h engineering
- [ ] Sprint 8 deliverable: Q3 cell-level result matching PG at
      SF=0.1 AND SF=1.0 in < 30s
