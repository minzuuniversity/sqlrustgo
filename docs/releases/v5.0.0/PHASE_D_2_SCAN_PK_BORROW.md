# Phase D.2 — Eliminate row clone in `scan_pk` PK lookup path

## Background

Phase D analysis (`PHASE_D_READ_ONLY_TPS_ANALYSIS.md`) Bottleneck #2:
on the PK lookup path, 55% of `execute_select` CPU is in `scan_pk`,
and 98% of that is `Vec<&[Value]>::clone` (which actually expands to
`Record = Vec<Value>` deep-clone including each `Value::Text` String).

Per query, 1-row match currently pays:
1. `scan_with_index` allocates a fresh `Vec<Record>` and pushes
   `data.rows[row_id].clone()` (deep clone).
2. `scan_pk` calls `rows.into_iter().next()` (already-owned move, free).
3. `engine_select.rs:1380` wraps with `row.map(|r| vec![r])` — another
   Vec alloc.

For a 1-row PK match that's 2 Vec allocs + 1 Record clone + N Value clones
(N = number of columns, each Text column pays a String::clone heap alloc).

## Fix

Add a new method `scan_pk_borrow` returning `Option<Cow<'_, Record>>`
that lets engines that can borrow from their in-memory row store skip
the clone. Engines that can't (or whose inner state outlives the lookup)
return `Cow::Owned(...)` and pay the clone.

`FileStorage::scan_pk_borrow` borrows `&data.rows[row_id]` directly
(no Vec alloc, no Record clone, no Value clone).

`MvccStorage::scan_pk_borrow` keeps the MVCC chain check, but if the
inner engine (FileStorage) returns `Borrowed`, MVCC stays in `Borrowed`
mode too (it just adds visibility validation without owning the row).

The trait signature stays compatible with `scan_pk` — `scan_pk` now
delegates to `scan_pk_borrow` and calls `into_owned()` for callers
that want ownership (preserves the existing API for any other callers).

## API

```rust
// crates/storage/src/engine.rs

/// Returns the row matching `pk` on the column named `pk_column`,
/// borrowed from the engine's in-memory state when possible.
///
/// `Cow::Borrowed(&row)` for engines that store rows in a long-lived
/// collection (e.g. `FileStorage` keeps `data.rows: Vec<Record>`).
/// `Cow::Owned(row)` for engines that materialize a row from
/// transient state (e.g. `MvccStorage` may rebuild the row from
/// the version chain on every call).
///
/// Phase D.2: added alongside `scan_pk` so read-only SELECT paths
/// can borrow and skip the row clone. Default impl delegates to
/// `scan_pk` and wraps in `Cow::Owned` (preserves old behavior
/// for engines that don't override).
fn scan_pk_borrow(
    &self,
    table: &str,
    pk_column: &str,
    pk: &Value,
) -> SqlResult<Option<Cow<'_, Record>>>;
```

## Caller change

`src/engine_select.rs` is the only caller of `scan_pk`:

```rust
let pk_value = ...;
let row = storage.scan_pk(lookup_table, &pk_column, &pk_value)?;
// row: Option<Record> (owned, cloned)
// ... becomes:
let row = storage.scan_pk_borrow(lookup_table, &pk_column, &pk_value)?;
// row: Option<Cow<'_, Record>> (borrowed or owned)
```

Then `row.map(|r| vec![r]).unwrap_or_default()` still works because
`Cow<'_, Record>` derefs to `&Record`, and `vec![r]` consumes the
Cow into a Vec of owned Record. **But** since `r` is a borrow, that
forces `into_owned()` implicitly. To keep the borrow alive through
the wrap, the caller does:

```rust
let row = storage.scan_pk_borrow(lookup_table, &pk_column, &pk_value)?;
let owned_row: Vec<Record> = row.map(|r| vec![r.into_owned()]).unwrap_or_default();
```

This is the SAME allocation count as before (1 outer Vec) — but the
inner Record no longer needs to be cloned first. The `into_owned()`
on a `Cow::Borrowed` is a single Vec::clone (cheap if the underlying
Record was already allocated).

Wait, that's no different. The actual win is: the **inner Engine no
longer clones the row** before returning it. Caller still needs to
own because `ExecutorResult.rows: Vec<Vec<Value>>` is owned.

So the real win is moving the clone out of the engine and into the
caller — but that's not actually faster in this case.

**Better approach: change ExecutorResult to support borrowed rows.**

That's too invasive. Instead:

**Even better: change the API to return `Option<&Record>` directly.**

The caller can then `.map(|r| r.to_vec())` to get an owned copy only
when it really needs one. The win: when the caller only needs to
inspect a few cells (e.g. for PK lookup result that goes straight
to wire encoding), the clone is skipped entirely.

But our caller (engine_select.rs) wraps in `vec![r]` because the
next stage (`apply_rls_filter`, projection, etc.) consumes `Vec<Record>`.

**Pragmatic path**: just inline the B+Tree search inside `scan_pk_borrow`
for FileStorage so we avoid the intermediate `Vec<Record>` alloc in
`scan_with_index`. The Record returned is still owned (via
`into_owned()` on the Cow at the caller), but the engine doesn't
allocate the intermediate Vec.

Even better: since the Cow is borrowed, if the caller can propagate
the borrow, no clone happens. Looking at engine_select.rs flow:

```rust
let row = storage.scan_pk(lookup_table, &pk_column, &pk_value)?;
self.instrumentation.on_seq_scan_start(lookup_table);
// AHI bookkeeping
let offset = row.as_ref().map(|_| 1).unwrap_or(0);
self.adaptive_hash_index.record_access(...);
row.map(|r| vec![r]).unwrap_or_default()  // <-- owns the row
```

The `.map(|r| vec![r])` consumes the Record into a Vec. This is the
ONLY point we need an owned Record. Before this point, the borrow
would suffice.

So: keep `scan_pk` returning owned `Option<Record>`, but **inline the
B+Tree search directly** in `scan_pk` (avoid `scan_with_index`'s
intermediate Vec alloc + clone). Clone once at the end.

The change:
- `scan_pk` in `FileStorage` does its own `index.search_all(...)`,
  gets the first row_id, returns `Some(data.rows[row_id].clone())`.
- `scan_with_index` is unchanged (still used for multi-row callers
  like `scan_pk_range`, batch lookup, etc.).

This skips:
- The intermediate `Vec<Record>` alloc in `scan_with_index`.
- The `.push(data.rows[i].clone())` loop in `scan_with_index`.
- The `into_iter().next()` overhead in `scan_pk`.

It still pays:
- `data.rows[row_id].clone()` — but only ONE clone instead of
  one in `scan_with_index` plus the wrapper overhead.

For PK index queries that match exactly 1 row (typical), this is
the optimal layout.

## Validation

- All existing storage tests must pass (race + integration + direct).
- Bench:
  - Phase D.1 baseline (PK=o_orderkey, PK B+Tree lookup): 4024 OPS
  - Phase D.2 expected: ~4500-5500 OPS (1.1-1.4× improvement)

## Files affected

- `crates/storage/src/file_storage.rs` (inline B+Tree search in `scan_pk`)
- `src/engine_select.rs` (no change if `scan_pk` keeps owned signature)
- (optional) `crates/storage/src/engine.rs` — could add a
  `scan_pk_borrow` variant for true borrowing; deferred unless
  inline doesn't yield enough improvement.

## Effort estimate

30 min: inline the B+Tree search into `scan_pk` directly. Re-run bench.

## Rollback plan

If the change regresses any test or shows no bench improvement, revert
the inline and instead add a `scan_pk_borrow` method that returns
`Option<&'a Record>` with `#[feature(generic_associated_types)]` or
similar — but only if profile data still shows scan_pk is the bottleneck.
