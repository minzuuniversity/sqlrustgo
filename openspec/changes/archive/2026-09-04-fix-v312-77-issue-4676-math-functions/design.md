## Context

Issue #4676: `MOD` / `POWER` / `LOG` / `EXP` / `SQRT` return NULL in `sqlrustgo`. The `eval_fn` function in `crates/executor/src/expr/mod.rs` defaults to `Value::Null` for unknown function names (line ~2182). These five standard SQL math functions are not registered in the match statement.

The fix is adding five new match arms. This is a pure executor expression change — no parser, storage, or WAL impact.

## Goals / Non-Goals

**Goals:**
- `MOD(x, y)` → `x % y` (floating-point remainder; returns `Value::Float`)
- `POWER(x, y)` → `x.powf(y)` (returns `Value::Float`)
- `SQRT(x)` → `x.sqrt()` (returns `Value::Float`)
- `LOG(x)` → `x.ln()` (natural logarithm; returns `Value::Float`)
- `EXP(x)` → `x.exp()` (returns `Value::Float`)
- NULL input → NULL output
- Domain errors (sqrt of negative, log of ≤0) → `Value::Null`

**Non-Goals:**
- `LOG(x, base)` (two-argument logarithm with explicit base): out of scope for v3.12
- `LN` / `LOG10` / `LOG2` as separate functions: can be aliases in a follow-up
- `MOD` with integer arguments returning Integer: keep consistent Float return type
- `POWER` with integer arguments returning Integer: keep Float

## Decisions

### D1. Float return type for all five functions

All five functions compute in f64 and return `Value::Float`. This is consistent with SQLite which uses IEEE 754 doubles for all numeric operations, and avoids type coercion complexity. The `Value::Float` rendering in `to_sql_string()` handles integral floats with `.0` suffix (fixed in issue #4721).

### D2. Domain errors return NULL, not NaN

SQLite returns NULL for domain errors (sqrt of negative, log of non-positive). We mirror this behavior: check `is_nan()` or `<= 0` before returning. This is fail-closed and matches SQLite compatibility.

### D3. `MOD(x, y)` uses `x % y` (f64 rem)

The `%` operator on f64 in Rust computes IEEE 754 remainder (truncated towards zero). SQLite uses `fmod`. Both are equivalent for positive divisors. For negative divisors, Rust and C fmod differ slightly — we use Rust's native `%` which is correct for the common case.

### D4. No new UDF registration needed

These are built-in functions in the `eval_fn` match, not user-defined functions. No changes to UDF registry or catalog.

## Risks / Trade-offs

- The change is adding five match arms. No new failure modes.
- All operations are standard IEEE 754 arithmetic. No performance concern.
- `LOG` without a base uses natural log — this matches PostgreSQL's `log()` and SQLite's `log()`. MySQL's `LOG(x)` also defaults to natural log. Good enough for v3.12.

## Verification Plan

- Unit test in `crates/executor/src/expr/mod.rs`: test each function with known values, NULL propagation, and domain errors.
- Integration test in `tests/integration/sql/`: test the CLI repro from the issue body.
- `cargo build --all-features` clean.
- `cargo test -p sqlrustgo-executor --lib` no regression.
- Manual CLI: `printf 'CREATE TABLE t(v REAL); INSERT INTO t VALUES (2), (4), (100); SELECT v, MOD(v, 3), POWER(v, 2), LOG(v), EXP(v/100), SQRT(v) FROM t;' | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` — first row must be `2,2.0,4.0,...,1.414...`.
