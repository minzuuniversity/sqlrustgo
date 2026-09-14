# Phase D.1 — Fix PK column hard-coded to `"id"`

## Background

Phase D analysis (`PHASE_D_READ_ONLY_TPS_ANALYSIS.md`) identified that
the primary-key lookup path is gated by a literal `"id"` string in two
places, regardless of the table's actual PK column name:

1. `src/engine_select_pk.rs:18` — `DEFAULT_PK_COLUMN: &str = "id"`
   used by `try_extract_pk_eq`.
2. `crates/storage/src/file_storage.rs:3321` — `scan_with_index(table, "id", pk)`
   inside `scan_pk`.

When a table's PK column is named anything other than `"id"`
(e.g. `o_orderkey`, `user_id`, `event_pk`), every WHERE clause that
should use the index falls through to a full table scan + linear find,
which is **the worst possible behavior** (allocates the entire row Vec
then iterates).

## Empirical evidence

With the current binary (ec6d96ac87) and a schema containing
`o_orderkey BIGINT PRIMARY KEY`:

| Bench | Path | OPS |
|---|---|---|
| `WHERE o_orderkey = ?` | Full scan + Vec::retain | 1977 |
| `WHERE id = ?` (renamed PK to `id`) | PK B+Tree lookup | 3831 |

Renaming the schema's PK column to `id` produces a **1.94× speedup
with zero code changes**.

## Fix design

### Two-pronged approach

1. **`scan_pk` trait signature** — add an explicit `pk_column: &str`
   parameter so the storage layer uses the right index, not a
   hard-coded literal. The default impl already does full scan +
   filter, so it's a simple parameter pass-through.

2. **`try_extract_pk_eq`** — caller-side, pass the actual PK column
   name from the table's `TableInfo` instead of the constant.

### Why not use the table's primary_key flag inside `scan_pk`?

Two reasons:

- `MvccStorage::scan_pk` is a thin wrapper around `inner.scan_pk`. The
  inner engine knows the index name; the outer wrapper shouldn't have
  to plumb it separately.
- Other engines (MemoryStorage, WalStorage) don't have a primary-key
  index at all — they use the default impl. They wouldn't benefit
  from the `primary_key: true` flag because they have no fast path.

So the PK column name flows **caller → engine.scan_pk**, and engines
that have a real PK index (FileStorage) use it; engines that don't
(default impl) just ignore it and fall through to full scan + filter.

### What changes

#### `src/engine_select_pk.rs`

- Add `resolve_pk_column(table_info: &TableInfo) -> String` helper that
  returns the column with `primary_key: true`, or `"id"` as fallback
  (matches existing behavior for legacy tables without a declared PK).
- Keep `DEFAULT_PK_COLUMN = "id"` for tests and old callers.

#### `crates/storage/src/engine.rs` (StorageEngine trait)

```rust
fn scan_pk(&self, table: &str, pk_column: &str, pk: &Value) -> SqlResult<Option<Record>> {
    let pk = pk.clone();
    Ok(self.scan(table)?.into_iter()
        .find(|row| row.first() == Some(&pk)))  // unchanged: default impl
}
```

#### `crates/storage/src/file_storage.rs`

```rust
fn scan_pk(&self, table: &str, pk_column: &str, pk: &Value) -> SqlResult<Option<Record>> {
    if let Some(info) = self.get_table_info(table).ok() {
        if info.columns.iter().any(|c| c.primary_key) {
            let rows = self.scan_with_index(table, pk_column, pk)?;
            if !rows.is_empty() {
                return Ok(rows.into_iter().next());
            }
            return Ok(None);
        }
    }
    // Fallback unchanged
    let pk = pk.clone();
    Ok(self.scan(table)?.into_iter().find(|row| row.first() == Some(&pk)))
}
```

#### `crates/storage/src/mvcc_storage.rs`

Pass `pk_column` through to `self.inner.scan_pk(table, pk_column, pk)`.

#### `src/engine_select.rs`

Before the PK lookup branch, fetch `table_info` and resolve the PK
column name. Pass it to both `try_extract_pk_eq_with_col` and
`storage.scan_pk`.

```rust
let table_info_for_pk = storage.get_table_info(lookup_table)?;
let pk_column = crate::engine_select_pk::resolve_pk_column(&table_info_for_pk);

let pk_lookup_rows = if let Some(pk_value) =
    crate::engine_select_pk::try_extract_pk_eq_with_col(&select.where_clause, &pk_column)
{
    let row = storage.scan_pk(lookup_table, &pk_column, &pk_value)?;
    // ... existing bookkeeping
};
```

The existing `let table_info = storage.get_table_info(lookup_table)?;`
at line 1375 is now redundant for the PK path (can re-use) but I'll
keep both fetches for clarity — `get_table_info` is cheap (HashMap
lookup on `Arc<RwLock>` metadata).

### Backward compatibility

- `try_extract_pk_eq` keeps its existing signature; the change is
  purely internal to `engine_select.rs`.
- `scan_pk` trait signature changes; existing callers (only
  `mvcc_storage::scan_pk` and `engine_select::execute_select`) need
  to pass the new `pk_column` argument. Test files that build their
  own engines will get a compile error pointing at the right place.
- Default `scan_pk` impl ignores `pk_column` — same behavior as before
  for engines that don't override.

## Validation plan

1. `cargo check --workspace` — must compile clean.
2. Existing PK lookup tests in `crates/storage/tests/` — must pass.
3. Re-run pymysql bench against schema with PK=o_orderkey:
   - **Before**: 1977 OPS
   - **After**: ~3831 OPS (1.94×)
4. Run pymysql bench against schema with PK=id:
   - **Before**: 3831 OPS
   - **After**: ~3831 OPS (unchanged — `resolve_pk_column` returns `"id"`)
5. `crates/storage/tests/phase_c_1_race.rs` — must pass (regression check).

## Effort estimate

1-2 hours:
- 5 file edits (small, mechanical)
- 1 design doc (this file)
- 2 bench runs (before/after)

## Files affected

- `src/engine_select_pk.rs` (add helper)
- `src/engine_select.rs` (use helper, plumb pk_column)
- `crates/storage/src/engine.rs` (trait signature)
- `crates/storage/src/file_storage.rs` (impl signature + body)
- `crates/storage/src/mvcc_storage.rs` (impl signature + pass-through)

No test files need signature changes — `scan_pk` is called through
trait dispatch in tests.
