# Capability: q15-subquery-from-verification

## ADDED Requirements

### Requirement: TPC-H Q15 (Global Suppliers Query) returns the correct 91 supplier rows after the Sprint 6 fixture regen

The executor MUST, when running Q15 against the Sprint 6 SF=0.1
fixture (60K lineitem rows, l_discount 11-value uniform, l_tax
9-value uniform, l_linenumber PK unique), return 91 rows matching
the PostgreSQL output row-for-row, modulo the cosmetic
f64-last-decimal-place difference in the s_total_revenue column.

#### Scenario: Q15 returns 91 rows post-Sprint 6 regen

- **GIVEN** the Sprint 6 fixture `/tmp/tpch_sf01_v3/` is loaded
      into the in-process storage (60K lineitem rows, 1K suppliers,
      200K customers, 20K orders, etc.)
- **WHEN** the executor runs the Q15 SQL form
      `SELECT s_suppkey, s_name, s_address, s_phone, s_total_revenue FROM supplier, (SELECT l_suppkey, SUM(l_extendedprice * (1 - l_discount)) AS s_total_revenue FROM lineitem WHERE l_shipdate >= '1995-01-01' AND l_shipdate < '1995-04-01' GROUP BY l_suppkey) AS revenue WHERE s_suppkey = revenue.l_suppkey ORDER BY s_total_revenue DESC;`
- **THEN** the result has 91 rows
- **AND** the row count matches `psql -U liying -d tpch_test -c "<q15.sql>"`

#### Scenario: Q15 cell values are byte-exact with PG modulo float-cosmetic

- **WHEN** the Q15 result is sorted by s_suppkey
- **AND** compared row-for-row with the PG output sorted by s_suppkey
- **THEN** the s_suppkey, s_name, s_address, s_phone columns are
      byte-exact
- **AND** the s_total_revenue column matches within ±0.001 (the
      Sprint 5 v2 acceptance "4 cosmetic" budget for f64 rounding)

#### Scenario: Q1-Q14, Q17, Q19 results unchanged after this change

- **WHEN** the executor runs Q1-Q14, Q17, Q19 against the same
      Sprint 6 fixture
- **THEN** the cell-level results match the pre-change baseline
      (no regression introduced)
