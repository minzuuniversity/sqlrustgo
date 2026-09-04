## Why

`sqlrustgo-cli sqlite` (HEAD, develop/v3.12.0) returns NULL for five standard SQL math functions — `MOD`, `POWER`, `LOG`, `EXP`, `SQRT` — because the `eval_fn` match arm in `crates/executor/src/expr/mod.rs` has no arms for these names and falls through to the default `Value::Null`. Issue #4676 reports this regression as production-blocking for scientific and financial calculations.

Repro:
```
$ printf 'CREATE TABLE t(v REAL); INSERT INTO t VALUES (2), (4), (100);
   SELECT v, MOD(v, 3), POWER(v, 2), LOG(v), EXP(v/100), SQRT(v) FROM t;' \
   | sqlrustgo-cli sqlite --batch --mode csv /tmp/db
2,,,,,,
4,,,,,
100,,,,,
```

Expected (SQLite / PostgreSQL / MySQL semantics):
```
2,2.0,4.0,0.693...,1.020...,1.414...
4,1.0,16.0,1.386...,1.040...,2.0
100,1.0,10000.0,4.605...,2.718...,10.0
```

Root cause: `eval_fn` (line ~2182) defaults to `Value::Null` for unknown function names. `MOD`, `POWER`, `LOG`, `EXP`, `SQRT` are not registered — no arms exist for them.

## What Changes

- Add 5 new match arms to `eval_fn` in `crates/executor/src/expr/mod.rs`:
  - `MOD(x, y)` → `x % y` (f64 remainder, SQLite/MySQL semantics)
  - `POWER(x, y)` → `x.powf(y)`
  - `LOG(x)` → `x.ln()` (natural log; note: `LOG(x, base)` is out of scope for v3.12)
  - `EXP(x)` → `x.exp()`
  - `SQRT(x)` → `x.sqrt()`
- All return `Value::Float`; NULL input propagates NULL.
- Domain errors (sqrt of negative, log of ≤0) return `Value::Null` (SQLite behavior).
- No AST change, no new public API, no storage/WAL change.

## Capabilities

### New Capabilities

- `executor-math-functions`: `sqlrustgo` MUST evaluate `MOD`, `POWER`, `LOG`, `EXP`, `SQRT` with f64 arithmetic and return `Value::Float` per the SQL standard. Domain errors return NULL.

### Modified Capabilities

- None.
