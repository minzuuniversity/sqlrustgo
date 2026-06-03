# F-09 Dual-Write Bug — Investigation Note

<!-- env:blocked:no-ci -->

> **Status**: KNOWN (not yet fixed in v3.8.0)
> **Created**: 2026-06-03
> **Investigator**: Hermes Agent
> **Triggered by**: test_insert_then_crash_rolls_back expecting 1 row, getting 2

## Summary

After applying the F-09 partial fixes (PR #2761) and deep fixes (PR #2764),
5 of 24 WAL tests still fail. All 5 failures have the same root cause:
**dual-write of autocommit DML** — once to disk via buffer flush, then
again from WAL replay.

## Reproduction

`test_insert_then_crash_rolls_back` setup:

```
1. CREATE TABLE t (id INTEGER, value TEXT)
2. BEGIN
3. INSERT INTO t VALUES (1, 'initial')   -- autocommit path (no implicit Begin)
4. COMMIT                                 -- flushes buffer to disk via save_table
5. BEGIN
6. INSERT INTO t VALUES (2, 'uncommitted') -- autocommit, buffer only (no flush)
7. drop(engine)                            -- crash before COMMIT
```

After `recover_and_rebuild`:

```
- engine2 = ExecutionEngine::with_wal_recovery(dir)
  - FileStorage::new_with_wal(dir) calls load_all_tables()    <-- key step
  - Tables already contain t with 1 row (from engine1 step 4 save_table)
  - recover_wal replays WAL Insert entry for t1 via force_insert
  - data.rows grows from 1 to 2
- SELECT COUNT(*) FROM t returns 2 (expected 1)
```

## Root cause (verified by backtrace tracing)

The first `insert_direct` in the test trace comes from
`engine.execute("COMMIT")` → `WalStorage::commit_transaction` → `inner.flush()` →
`FileStorage::flush_all_buffers` → `flush_buffer` → `insert_direct`. This writes
the buffer's row to `data.rows` AND calls `save_table` which persists to disk.

When `recover_and_rebuild` runs, `FileStorage::new_with_wal` calls
`load_all_tables()` which reads the disk JSON files into the `tables` HashMap.
**The row that was flushed-and-saved is now in the new FileStorage's `tables[t].rows`.**

Then `recover_wal` replays the WAL Insert entry via `force_insert`, which calls
`insert_direct` again, extending `data.rows` to 2 rows. The replayed row
duplicates the one already loaded from disk.

## Why PR #2761 + PR #2764 don't fix this

PR #2761 introduced `force_insert` and the span-based `filter_committed_entries`.
PR #2764 added `engine.begin_transaction` delegation to `storage.begin_transaction`
and `replace_by_key` use of `force_insert`. These fixes correctly handle:

- `Begin`/`Commit`/`Rollback` span detection
- `force_insert` bypassing `insert_buffer`
- Same-transaction visibility (`scan` merges buffer)

But none of them address the **dual-write**:

1. Autocommit INSERT → buffer (1 row in buffer)
2. COMMIT (explicit) → flush buffer → `data.rows += 1` + `save_table` (persists to disk)
3. drop → recover → new FileStorage loads tables from disk → `data.rows = 1` (loaded)
4. recover_wal replays WAL Insert entry → `force_insert` → `data.rows = 2` (replayed duplicate)

## Possible fix directions (not yet implemented)

### Option 1: Replay dedup with disk state
After loading tables, walk through replay entries and check if the row already
exists in `data.rows` (by primary key). Skip insert if duplicate. Adds runtime
cost but minimal logic.

### Option 2: Don't load tables in `new_with_wal`
If `recover_wal` is going to run anyway, don't pre-load tables. But CREATE TABLE
doesn't go through WAL (DDL is not logged), so this would break schema recovery.

### Option 3: Log DDL to WAL
Make `execute_create_table` write a synthetic WAL entry (e.g. `WalEntryType::DDL`)
so recover can rebuild schema from WAL alone. Then `new_with_wal` doesn't need
to load tables.

### Option 4: Don't flush buffer on COMMIT
Currently COMMIT flushes the buffer, causing dual-write. If COMMIT only writes
the Commit WAL entry and leaves the buffer in place, then drop + recover would
replay the buffered inserts via WAL. But this changes transaction semantics
(buffer content would only become visible after `commit_transaction` returns).
This needs careful design.

### Option 5: Use a "replay marker" in WAL
Each COMMIT entry could include a "data was flushed" flag. On replay, if the
flag is set, skip the contained DML entries (they're already on disk).

## Recommendation

**Option 1** is the smallest, safest fix. Add dedup logic to
`apply_entry` for `WalEntryType::Insert`:

```rust
if let Some(existing) = storage.scan(&table_name)?.iter().find(|r| r == &record) {
    return Ok(()); // already on disk, skip replay
}
```

This costs an O(n) scan per replayed insert but is correct and minimal. The
"replay + dedup" model is a standard pattern in database recovery (WAL replay
on systems with periodic checkpoints).

For higher throughput, add a "LSN watermark" to the FileStorage (highest LSN
already applied). On replay, skip entries with LSN <= watermark. This requires
storing the watermark to disk on `save_table`.

## Affected tests

These 5 tests fail with "expected 1 row got N" because of the dual-write:

- `test_begin_then_crash_rolls_back`
- `test_insert_then_crash_rolls_back`
- `test_partial_insert_write_recovery`
- `test_partial_commit_flush_recovery`
- `test_commit_flush_crash_replays`

All have the same pattern: autocommit INSERT → COMMIT (or autocommit-only) → drop → recover.

## References

- PR #2761: storage layer fixes (force_insert trait, span-based filter, scan merge)
- PR #2764: deep fixes (begin_transaction delegation, replace_by_key force_insert, autocommit-aware filter)
- Issue #2741: UPDATE replay value preservation
- BETA_GATE_REPORT.md: F-09 marked NOT_DONE
