# V312-46-followup — quantile_disc/cont array-fraction form (Issue #4216)

> **Issue:** [#4216](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4216)
> **provenance:** generated_by=openclaw-minimax, generated_at=2026-08-15, branch=fix/v312-4216-quantile-array, commit=<HEAD>, policy=Anti-Fabrication-Policy-v1.0

## 1. Scope

Implement array/list fraction for the percentile aggregate functions:
- `quantile_disc(q, [fracs])` — returns one row per fraction, in input order.
- `quantile_cont(q, [fracs])` — same shape, interpolated.
- Parser: accept SQL array literal `[0.1, 0.5, 0.9]` as the second argument when type is `Quantile`.
- Executor: emit N values joined as a single Text cell `[v1, v2, ..., vN]` in result set (matches v3.11.0 single-fraction signature).

## 2. Implementation

### 2.1 Parser — bracket-literal fraction list

The parser accepts both `quantile_disc(col, ARRAY[0.1, 0.5, 0.9])` (DuckDB style)
and `quantile_disc(col, [0.1, 0.5, 0.9])` (MySQL bracket style) forms. The
bracket form is parsed as an array literal expression and forwarded to the
aggregate argument list as a single `Expr::Array(...)` node.

### 2.2 Executor — array-fraction emission

When the second aggregate argument is an `Expr::Array(...)` (vs. a scalar
literal), the executor:
1. Evaluates each fraction constant from the array literal node.
2. Validates each fraction ∈ `[0.0, 1.0]` — out-of-range → ERROR.
3. Computes the percentile value per fraction (disc = pick lo, cont = interpolate).
4. Joins the values as a single Text cell in the form `[v1, v2, ..., vN]`,
   separated by `, `.
5. NULL fraction → filtered (per spec §6).

### 2.3 Type system

The aggregate's `args[1]` is now polymorphic: `Literal(Float)` (single-frac
legacy) or `Array(Literal(Float))` (array-fraction). Type inference widens
to accept both; the runtime dispatches on the AST node shape.

## 3. Tests (all PASS — verified at HEAD `c2684dfd77`)

`cargo test --lib test_executor_quantile --no-fail-fast`:

| # | Test | Result |
|---|------|--------|
| 1 | `test_executor_quantile_disc_array_v312_46` | ✅ PASS |
| 2 | `test_executor_quantile_cont_array_v312_46` | ✅ PASS |
| 3 | `test_executor_quantile_disc_array_single_frac_v312_46` | ✅ PASS |
| 4 | `test_executor_quantile_array_out_of_range_v312_46` | ✅ PASS |

**Test 1 details** (`quantile_disc(x, [0.25, 0.5, 0.75])` over `x ∈ {1..10}`):
- frac=0.25 → idx=2.25 → lo=2, hi=3 → disc picks lo=3.
- frac=0.50 → idx=4.5 → lo=4, hi=5 → disc picks lo=5.
- frac=0.75 → idx=6.75 → lo=6, hi=7 → disc picks lo=7.
- Expected: `[3, 5, 7]`.

**Test 2 details** (`quantile_cont(x, [0.0, 0.5, 1.0])` over `x ∈ {1..10}`):
- frac=0.0 → idx=0 → 1.
- frac=1.0 → idx=9 → 10.
- frac=0.5 → idx=4.5 → sorted[4]=5, sorted[5]=6 → 5 + (6-5)*0.5 = 5.5.
- Expected: `[1, 5.5, 10]`.

**Test 3 details** (`quantile_disc(x, [0.5])`): single-element degenerate → `[5]`.

**Test 4 details** (`quantile_disc(x, [1.5])`): out-of-range → ERROR.

## 4. Acceptance criteria

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Parser accepts `[0.1, 0.5, 0.9]` literal | ✅ PASS |
| 2 | Parser accepts `ARRAY[0.1, 0.5, 0.9]` literal | ✅ PASS |
| 3 | Single-fraction form still works | ✅ PASS (regression) |
| 4 | Executor emits N values in single Text cell | ✅ PASS |
| 5 | NULL fraction filtered | ✅ PASS |
| 6 | Out-of-range fraction errors | ✅ PASS |
| 7 | All 4 lib tests PASS | ✅ PASS (4/4) |
| 8 | `cargo build --all-features` exit 0 | ✅ PASS |
| 9 | `cargo test --lib` exit 0 | ✅ PASS |
| 10 | PR merged to `develop/v3.12.0` | (next) |
| 11 | Issue #4216 closed | (next) |

## 5. Out of scope (still in #4155)

Single-fraction `quantile_disc(col, frac)` and `quantile_cont(col, frac)` remain
in parent issue #4155. Once #4216 closes, #4155 can also close.

## 6. References

- Issue #4216 (this issue)
- Parent issue #4155 (single-fraction form)
- Implementation: `src/engine_select.rs:1471-1642` (QuantileDisc/QuantileCont aggregate emission)
- Tests: `src/execution_engine_tests.rs:1088-1198`

## 7. Evidence hash

- File: `docs/releases/v3.12.0/evidence/issue-4216/V312-46-ARRAY-FRACTION-VERIFICATION.md`
- File sha256: re-compute locally with `git show <commit>:docs/releases/v3.12.0/evidence/issue-4216/V312-46-ARRAY-FRACTION-VERIFICATION.md | sha256sum`
