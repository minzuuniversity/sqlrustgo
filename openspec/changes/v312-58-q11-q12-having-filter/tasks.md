# Tasks — V312-58 Q11/Q12 HAVING Filter Fix

## 1. Investigation

- [x] 1.1 Confirm parser produces HAVING in AST (`s.having: Option<Expression>` at parser.rs:588)
- [x] 1.2 Confirm no executor code references `having` (root cause: silent drop)
- [x] 1.3 Inventory aggregate evaluation pipeline (`crates/executor/src/sql_executor.rs`)
- [x] 1.4 Inventory aggregate scope evaluation (`crates/executor/src/expr/mod.rs::eval_aggregate_lookup`)

## 2. Implementation

- [ ] 2.1 Locate the GROUP BY execution site in `sql_executor.rs`
- [ ] 2.2 Add HAVING filter step: for each group, evaluate `s.having` against aggregate scope
- [ ] 2.3 Use `eval_aggregate_lookup` to resolve aggregate references in HAVING
- [ ] 2.4 Filter groups: keep only those where HAVING evaluates to `true`
- [ ] 2.5 Handle 3-valued logic: `false` AND `unknown` → exclude

## 3. Tests

- [ ] 3.1 Unit test: HAVING with COUNT(*) > N
- [ ] 3.2 Unit test: HAVING with SUM(a*b) > N (Q11 pattern)
- [ ] 3.3 Unit test: HAVING with AND of aggregates
- [ ] 3.4 Unit test: HAVING with OR
- [ ] 3.5 Integration test: Q11 on tpch-tiny (verify < total partsupp groups)
- [ ] 3.6 Integration test: Q12 simplified form still PASS

## 4. Validation

- [ ] 4.1 `openspec validate v312-58-q11-q12-having-filter` PASS
- [ ] 4.2 Existing tpch_sf01_inprocess_test still PASS (regression)
- [ ] 4.3 No regression in check_beta_v3.12.0.sh B1-B8 gates

## 5. PR & close

- [ ] 5.1 Branch: `fix/v312-58-q11-q12-having-filter` from `develop/v3.12.0`
- [ ] 5.2 Commit message `[V312-58]` prefix
- [ ] 5.3 Push + PR to develop/v3.12.0
- [ ] 5.4 Reference #4377 + #4378 in PR body
- [ ] 5.5 Note: SF=1 row count parity verification requires SF=1 fixture (out of scope)

## 7. Known limitations

- **SF=1 fixture not available locally** — full Q11/Q12 row-count parity requires `/tmp/tpch-sf1/*.tbl`
- **tpch-tiny only verifies** that HAVING executes correctly (output is non-empty and smaller than no-HAVING). For exact row count (29,636 for Q11 GERMANY, 2 for Q12), SF=1 fixture is required.
- This change only fixes **HAVING execution**. Other TPC-H bugs (Q2 LIMIT, Q7 EXTRACT, Q17/Q20/Q22 TIMEOUT) remain tracked in their own issues.