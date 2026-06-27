# q16-not-in-subquery-verification Specification

## Purpose
TBD - created by archiving change 2026-06-08-v390-sprint6-q16-not-in. Update Purpose after archive.
## Requirements
### Requirement: TPC-H Q16 (Parts/Supplier Relationship Query) returns the correct 284 rows after the Sprint 6 fixture regen

The executor MUST, when running Q16 against the Sprint 6 SF=0.1
fixture (60K lineitem rows, 200K parts, 800K partsupp rows, 1K
suppliers), return 284 rows matching the PostgreSQL output
row-for-row, with byte-exact p_brand, p_type, p_size, and
supplier_cnt columns.

#### Scenario: Q16 returns 284 rows post-Sprint 6 regen

- **GIVEN** the Sprint 6 fixture is loaded into the in-process
      storage
- **WHEN** the executor runs the Q16 SQL form (NOT IN subquery on
      supplier + GROUP BY p_brand, p_type, p_size)
- **THEN** the result has 284 rows
- **AND** the row count matches `psql -U liying -d tpch_test -c "<q16.sql>"`

#### Scenario: Q16 cell values are byte-exact with PG

- **WHEN** the Q16 result is sorted by (p_brand, p_type, p_size)
- **AND** compared row-for-row with the PG output sorted the same way
- **THEN** all 4 columns (p_brand, p_type, p_size, supplier_cnt)
      are byte-exact

#### Scenario: Q1-Q15, Q17, Q19 results unchanged after this change

- **WHEN** the executor runs Q1-Q15, Q17, Q19 against the same
      Sprint 6 fixture
- **THEN** the cell-level results match the pre-change baseline
      (no regression introduced)

