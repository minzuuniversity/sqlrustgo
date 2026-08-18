# V313-followup-2 / #4155 quantile single-fraction Closure Evidence

> **provenance:** generated_by=claude, generated_at=2026-08-18T23:36:00+08:00, commit=122caf018, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

## Acceptance Criteria Verification

### 1. Parser: AggregateFunction::QuantileDisc and QuantileCont variants

**Status**: ✅ PASS

```rust
// crates/parser/src/parser.rs:521-526
QuantileDisc,   // V313-followup-2 / Issue #4155
QuantileCont,   // V313-followup-2 / Issue #4155
```

Parser correctly recognizes `quantile_disc(...)` and `quantile_cont(...)` via `parse_aggregate_function`.

### 2. Executor: single value (floor index) for quantile_disc, interpolated for quantile_cont

**Status**: ✅ PASS

```rust
// src/engine_select.rs:1471-1541
// quantile_disc(col, frac) → single value (floor index)
// quantile_cont(col, frac) → linearly interpolated value
```

### 3. Dispatch: QuantileDisc / QuantileCont branches in aggregate dispatch

**Status**: ✅ PASS

Aggregate dispatch has branches for QuantileDisc and QuantileCont.

### 4. Unit tests

**Status**: ✅ PASS

```
$ cargo test --test null_handling_test green_4155
running 2 tests
test tests::green_4155_quantile_disc_single_fraction ... ok
test tests::green_4155_quantile_cont_single_fraction ... ok
test result: ok. 2 passed
```

### 5. Fixture note

The 3 fixture files were updated with scope comments explaining deferred items. Array-fraction form is tracked in #4216.

## All Tests Pass

```
$ cargo test --lib 2>&1 | grep "test result"
test result: ok. 55 passed; 0 failed
```

## Pull Requests

- PR #4332: fix: TPC-H SF=1 zero-row issues (Q5/Q8/Q9/Q10, NOT EXISTS engine bug)
- Related: quantile implementation commits in develop/v3.12.0
