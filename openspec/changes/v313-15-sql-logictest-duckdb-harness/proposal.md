## Why

V313-15 tracks deferred DuckDB harness fixes for SET variable parsing from V312-11 smoke baseline.

## Deferred Items
- aggregate__quantile_fun.test: set variable sf 0.001
- sql__quantile_fun.test: set variable sf 0.001
- quantile_fun.test: set variable sf 0.001

## Acceptance Criteria
- [ ] aggregate__quantile_fun.test: PASS
- [ ] sql__quantile_fun.test: PASS
- [ ] quantile_fun.test: PASS
