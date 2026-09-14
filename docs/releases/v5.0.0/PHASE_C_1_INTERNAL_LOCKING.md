# v5.0.0 Phase C.1 — FileStorage internal locking design

**Author**: sqlrustgo core team
**Date**: 2026-09-14
**Status**: Design document for Phase C.1 implementation
**Parent doc**: `docs/releases/v5.0.0/PHASE_C_SERVER_LOCK_REMOVAL.md`
**Goal**: Make every inherent `impl FileStorage` method callable on `&self` so
that Phase C.2 can replace `Arc<RwLock<FileStorage>>` with `Arc<FileStorage>`
at the server layer.

## 1. Scope correction vs. parent doc

The parent doc states:

> Convert every `&mut self` method (34 total) to `&self`, taking `write_lock`
> internally only when mutating `tables` or `insert_buffer`.

A full audit of `crates/storage/src/file_storage.rs` on `e14f519b2e` shows
this number is wrong:

| Block | `&mut self` methods | Scope under C.1 |
|-------|---------------------|------------------|
| `impl FileStorage` (inherent, line 103) | 9 | **rewrite to `&self`** |
| `impl FileStorage` (inherent, line 2841) | 4 | **rewrite to `&self`** |
| `impl FileStorage` (inherent, line 2932) | 1 | **rewrite to `&self`** |
| `impl FileStorage` (inherent, line 4032) | 1 | **rewrite to `&self`** |
| `impl FileStorage` (inherent, line 4086) | 2 | **rewrite to `&self`** |
| `impl StorageEngine for FileStorage` (line 2945) | 25 | **signature frozen** (trait constraint) |
| **Total `&mut self` methods** | **45** | 20 rewritable + 25 frozen |

The parent doc's "34" likely undercounted either the inherent block or the
trait impl. The **25 trait impl methods cannot be rewritten** because the
`StorageEngine` trait in `crates/storage/src/engine.rs` declares them as
`&mut self` (39 `&mut self` methods on the trait, 25 of them implemented
specifically by `FileStorage`). Changing the trait signature would force
synchronous changes in 7 other engines (`MemoryStorage`, `WalStorage`,
`BinaryStorage`, `AppendOnlyStorage`, `BinaryStorageV2`, `ColumnarStorage`,
`MvccStorage`) and ~180 external call sites — out of scope for C.1.

The agreed trade-off (per 2026-09-14 decision): **C.1 only rewrites the 20
inherent methods, leaves the 25 trait impl signatures untouched, and adds
internal `unsafe` bridging inside the trait impl methods** so the existing
trait signature still satisfies the borrow checker while the underlying
state is internally synchronised.

## 2. Field ownership model after C.1

Current `FileStorage` (line 19) mixes unprotected and protected fields:

```rust
pub struct FileStorage {
    data_dir: PathBuf,                              // immutable after init
    tables: HashMap<String, TableData>,             // ← UNPROTECTED
    indexes: RwLock<HashMap<(String, String), BPlusTree>>,         // ✓ internal
    index_metadata: RwLock<HashMap<String, IndexInfo>>,            // ✓ internal
    insert_buffer: HashMap<String, Vec<Record>>,     // ← UNPROTECTED
    buffer_threshold: usize,                        // immutable after init
    enable_buffer: bool,                            // immutable after init
    current_tx_id: u64,                             // ← UNPROTECTED
    tx_undo_log: Vec<UndoOp>,                       // ← UNPROTECTED
    triggers: RwLock<HashMap<String, TriggerInfo>>, // ✓ internal
    views: RwLock<HashMap<String, ViewInfo>>,       // ✓ internal
    gap_lock_manager: Option<Arc<GapLockManager>>,
    dirty_tables: HashSet<String>,                  // ← UNPROTECTED
}
```

After C.1, the five unprotected fields become protected by a single
`parking_lot::Mutex<()>` (the `write_lock`):

```rust
pub struct FileStorage {
    // ... existing protected fields stay as-is ...
    /// C.1: serialises all writes to {tables, insert_buffer, dirty_tables,
    /// current_tx_id, tx_undo_log}. Reads of these fields are lock-free
    /// when no writer holds the lock (the read path clones `data.rows`
    /// and `insert_buffer` slices, so it is naturally torn-read-free).
    ///
    /// Why `Mutex<()>` not `RwLock<()>`: every read path also needs to
    /// see a coherent `tables` map (a concurrent `insert_table` could
    /// otherwise be missed), so we serialise readers too. The hot path
    /// cost is one atomic CAS (~20-30ns uncontended) — same as the
    /// `Arc<RwLock<S>>` it replaces.
    write_lock: parking_lot::Mutex<()>,
}
```

**Why a single mutex, not five separate locks**: the fields are accessed
together by `insert`, `delete`, `commit_transaction`, `flush`, etc.
Splitting into five locks would require multi-lock acquisition ordering
and risk deadlocks under the trait impl's `&mut self` constraint. One
mutex mirrors the granularity of the outer `Arc<RwLock<S>>` we are
eventually replacing in C.2 — minimal new contention surface.

**Why `Mutex<()>` and not per-field `Mutex<HashMap>`**: the surrounding
struct holds the hashmap entries by value, and converting to
`Mutex<HashMap>` would force every read site to do `lock.read()` (an
atomic CAS). A `Mutex<()>` lets reads stay lock-free on the bare
`HashMap`/`HashSet` while writes briefly lock. The `Mutex<()>` is only
acquired by writers — readers race against writers but get either the
old or new state (no torn reads because each `Record` is a `Vec<Value>`
clone, not a pointer into the hashmap's internal bucket array).

## 3. Unsafe bridging inside `impl StorageEngine for FileStorage`

The 25 trait impl methods keep `&mut self` (Rust enforces this) but their
bodies now need to mutate the internally-protected fields. They do so via
a private `unsafe fn` helper:

```rust
impl FileStorage {
    /// C.1: temporarily acquire the write_lock and reborrow `self` as
    /// `&mut Self`. Caller MUST NOT hold any other reference into
    /// `self.tables` / `self.insert_buffer` / `self.dirty_tables` /
    /// `self.current_tx_id` / `self.tx_undo_log` across the call.
    ///
    /// Safety: `write_lock` is a `Mutex<()>`, so only one writer can
    /// reborrow `self` mutably at a time. Any reader that holds a `&self`
    /// borrow into one of the protected fields would race; this is
    /// prevented by the borrow checker (the caller can only enter this
    /// helper with `&mut self`, which means no `&self` borrow is live).
    #[allow(clippy::mut_mut)]
    unsafe fn with_write_lock<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let _guard = self.write_lock.lock();
        f(self)
    }
}
```

Trait impl methods that need to mutate protected fields wrap their body:

```rust
fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
    unsafe {
        self.with_write_lock(|s| {
            if s.in_transaction() {
                s.insert_buffered(table, records)?
            } else if !s.enable_buffer {
                s.insert_direct(table, records)?
            } else {
                s.insert_buffered(table, records)?
            };
            s.dirty_tables.insert(table.to_string());
            Ok(())
        })
    }
}
```

The `unsafe` block is unavoidable — `&mut self → &self → &mut self`
reborrows are exactly what Rust's aliasing rules forbid, and we are
opting out under a manually-maintained invariant (the `Mutex<()>`).
This is the same pattern `parking_lot::Mutex<T>::lock()` uses internally
to return a `MutexGuard<T>` that derefs to `&mut T`.

### 3.1 Auditable invariant checklist (one bullet per call site)

The audit covers all 53 protected-field access points inside the trait
impl (line 2945..4031). For each, the design classifies whether the call
is a **read** (no `write_lock` needed — the `HashMap`/`HashSet` read is
racy-but-safe because each `Record` is owned and clone is atomic) or a
**write** (must wrap in `with_write_lock`).

| Field | Read sites (no lock) | Write sites (need `with_write_lock`) |
|-------|----------------------|--------------------------------------|
| `tables` | L3014 `.keys()`, L3103 `.get()`, L3135 `.get()`, L3519 `.get().cloned()`, L3534 `.contains_key()`, L3538 `.keys()` | L3238 `.get_mut()`, L3316 `.get_mut()`, L3400 `.get_mut()`, L3420 `.get_mut()`, L3469 `.get_mut()`, L3637 `.get_mut()`, L3663 `.remove()`, L3674 `.insert()` |
| `insert_buffer` | L3030 `.get()`, L3050 `.get()`, L3112 `.get()`, L3172 `.get()` | L3015 `.get_mut()`, L3294 `.remove()`, L3295 `.get_mut()`, L3387 `.remove()`, L3388 `.get_mut()`, L3530 `.clear()` |
| `dirty_tables` | (none) | L3219 `.insert()`, L3286 `.insert()`, L3381 `.insert()`, L3406 `.insert()`, L3458 `.insert()`, L3488 `.insert()`, L3517 `mem::take` |
| `current_tx_id` | L2947, L2951, L2976, L2978, L2982, L2986, L3001, L3232, L3313, L3424 (all reads) | L2955, L2980, L2996, L3019 (writes) |
| `tx_undo_log` | (none — every site mutates) | L2981 `.clear()`, L2995 `.clear()`, L3008 `.pop()`, L3247/3258/3323/3334/3442 `.push()` |

**Read sites do NOT need `with_write_lock`**: every read of `tables` is
followed by `.clone()` (e.g. `data.rows.clone()`), and every read of
`insert_buffer` produces an iterator of `Record` clones. A concurrent
write that inserts/removes a key in the HashMap is observable (a newly
inserted key may or may not be visible), but no reader sees a torn
record — `Record` is a `Vec<Value>` and `Vec::clone()` is atomic on the
allocation level. This matches the existing `WalStorage` wrapper pattern.

**Note on `current_tx_id`**: reads happen in the hot path (`in_transaction()`
is called from every query). The current `u64` read is naturally atomic on
x86_64 / aarch64 (8-byte aligned loads/stores). We do NOT wrap reads in
`with_write_lock`; this preserves the no-extra-lock-acquire property for
the read path that the whole C.1 effort is trying to achieve.

## 4. Method-by-method plan (20 inherent rewrites)

### 4.1 Block 1: `impl FileStorage` (line 103)

| Line | Method | Rewrite |
|------|--------|---------|
| L320 | `load_all_views(&mut self)` | `&self`; internalises `self.views.write()` (already RwLock-protected, just drop `&mut` on outer) |
| L381 | `load_all_triggers(&mut self)` | `&self`; same as above for `triggers` |
| L410 | `load_all_tables(&mut self)` | `&self`; mutates `tables` — needs `with_write_lock` |
| L432 | `load_all_indexes(&mut self)` | `&self`; `indexes.write()` already internal |
| L549 | `get_table_mut(&mut self, name)` | keep `&mut self` — returns `&mut TableData`, the caller needs exclusive access; OR replace with a closure-based `with_table_mut(&self, name, \|t\| ...)` API (preferred, avoids the API break) |
| L554 | `insert_table(&mut self, name, data)` | `&self`; mutates `tables` + calls `save_table` (file I/O) — needs `with_write_lock` |
| L560 | `drop_table(&mut self, name)` | `&self`; mutates `tables` + deletes file — needs `with_write_lock` |
| L578 | `flush(&mut self)` | `&self`; mutates `dirty_tables`, reads `tables` — needs `with_write_lock` (write to `dirty_tables`) |
| L595 | `create_database(&mut self, name)` | `&self`; pure FS, but signature should drop `&mut` for consistency |
| L602 | `drop_database(&mut self, name)` | `&self`; same |
| L752 | `drop_index(&mut self, table, col)` | `&self`; `indexes.write()` already internal |

### 4.2 Block 2: `impl FileStorage` (line 2841) — buffer mgmt

| Line | Method | Rewrite |
|------|--------|---------|
| L2842 | `insert_direct(&mut self, table, records)` | `&self`; mutates `tables` + `save_table` — needs `with_write_lock` |
| L2851 | `insert_buffered(&mut self, table, records)` | `&self`; mutates `insert_buffer` — needs `with_write_lock` |
| L2861 | `flush_buffer(&mut self, table)` | `&self`; mutates `insert_buffer` + calls `insert_direct` — needs `with_write_lock` |
| L2868 | `flush_all_buffers(&mut self)` | `&self`; loops over `insert_buffer` keys, calls `flush_buffer` — needs `with_write_lock` |

### 4.3 Block 3: `impl FileStorage` (line 2932) — clean slate

| Line | Method | Rewrite |
|------|--------|---------|
| L2882 | `discard_all_buffers(&mut self)` | `&self`; mutates `insert_buffer` — needs `with_write_lock` |
| L2937 | `clear_all_tables(&mut self)` | `&self`; mutates `tables` + `insert_buffer` — needs `with_write_lock` |

### 4.4 Block 4: `impl FileStorage` (line 4032) — parallel flush

| Line | Method | Rewrite |
|------|--------|---------|
| L4035 | `flush_parallel(&mut self)` | `&self`; mutates `dirty_tables` — needs `with_write_lock` |

### 4.5 Block 5: `impl FileStorage` (line 4086) — tx support

| Line | Method | Rewrite |
|------|--------|---------|
| L4090 | `next_tx_id(&mut self)` | `&self`; reads `tx_undo_log` — no `with_write_lock` needed (read only) |
| L4105 | `apply_undo(&mut self, op)` | `&self`; mutates `tables` + `dirty_tables` — needs `with_write_lock` |

**Total**: 20 inherent methods. 14 need `with_write_lock`; 6 are read-only or already use internal `RwLock` and just drop `&mut` from the signature.

## 5. Caller-side migration

After C.1.1 (inherent rewrite), 6 external files have call sites that
need updating. The compiler will guide this — every `let mut s = ...; s.foo()`
where `foo` is now `&self` will become `let s = ...; s.foo()`. The
explicit list:

| External file | Methods called | Mutation needed |
|---------------|----------------|------------------|
| `crates/storage/src/binary_storage.rs` | `load_all_tables` (1) | drop `&mut` on caller-side binding |
| `crates/storage/tests/file_storage_direct_v3_12.rs` | `get_table_mut`, `insert_table`, `flush_all_buffers`, `clear_all_tables`, `flush_parallel` (5+) | drop `&mut`, switch `get_table_mut` to closure API |
| `crates/storage/tests/storage_integration_test.rs` | `insert_table` (1) | drop `&mut` |
| `crates/storage/tests/bulk_load_quadraticity.rs` | `insert_direct`, `insert_buffered`, `flush_buffer` (3) | drop `&mut` |
| `crates/storage/benches/storage_benchmark.rs` | `flush_all_buffers` (1) | drop `&mut` |
| `crates/storage/tests/e2e_crash_recovery_proof.rs` | `flush_all_buffers` (1) | drop `&mut` |
| `crates/tools/src/backup.rs` | `drop_table` (1) | drop `&mut` |
| `crates/server/src/openclaw_endpoints.rs` | `drop_table`, `drop_index` (2) | drop `&mut` |
| `crates/storage/src/engine.rs` | `create_database`, `drop_database`, `discard_all_buffers`, `flush_parallel` (4) | drop `&mut` |
| `crates/storage/src/wal_storage.rs` | `discard_all_buffers` (1) | drop `&mut` |
| `crates/storage/src/mvcc_storage.rs` | `create_database`, `drop_database`, `discard_all_buffers` (3) | drop `&mut` |
| `crates/transaction/src/transaction_manager.rs` | `next_tx_id` (1) | drop `&mut` |
| `crates/transaction/src/mvcc.rs` | `next_tx_id` (1) | drop `&mut` |

Note: most of these callers do NOT hold `Arc<RwLock<FileStorage>>` — they
construct a `FileStorage` directly in test/bench context and call methods
on `&mut FileStorage`. The change `&mut self → &self` means the *caller*
no longer needs `let mut storage`. Rust's borrow checker will reject
the `let mut` (with `unused_mut` warning) — `-D warnings` would turn
this into a CI break, so we must fix every caller-side `let mut`.

## 6. Test plan

C.1 changes a lot of signatures and adds unsafe. The test plan must catch
both correctness regressions and concurrency races.

### 6.1 Correctness

```
cargo build --workspace
cargo test --lib -p storage                    # 141 existing tests
cargo test --lib --workspace                   # full lib suite
cargo test --test file_storage_direct_v3_12    # 32 existing FileStorage tests
cargo test --test wal_storage_direct_v3_12     # 32 existing WalStorage tests
```

### 6.2 Concurrency (NEW tests, added under `crates/storage/tests/phase_c_1_race.rs`)

```
test c1_concurrent_scans_no_torn_reads        — 8 threads × 10s × scan()
test c1_concurrent_insert_and_scan            — 4 writer + 4 reader threads, 10s
test c1_concurrent_flush_and_scan             — 4 flusher + 4 reader threads, 10s
test c1_concurrent_begin_commit_rollback      — 8 threads × 100 tx each
test c1_data_integrity_after_concurrent_ops   — final SELECT COUNT(*) matches expected
```

The first three tests are the regression catchers for the unsafe bridging —
if `with_write_lock` is missing on a write path, `cargo test --release
c1_concurrent_*` should produce a corrupted row count within seconds.

### 6.3 Performance (informational, not gating C.1)

Run the existing `oltp_read_only` microbench on `feat/v5.0.0-phase-c`
(after C.1.3 merges) — expected: same or slightly worse than baseline
(C.1 alone does NOT yet remove the outer `Arc<RwLock<S>>`, so it cannot
improve TPS). The actual perf win lands after C.2.

## 7. Commit plan

| SHA | Commit | Scope |
|-----|--------|-------|
| C.1.0 (this doc) | `docs(v5.0.0): Phase C.1 internal locking design` | this file only |
| C.1.1 | `refactor(storage): FileStorage inherent methods → &self + write_lock` | file_storage.rs struct field + 20 inherent methods |
| C.1.2 | `refactor(storage): trait impl bodies wrapped in with_write_lock` | 53 access sites across the 25 trait impl methods |
| C.1.3 | `test(storage): Phase C.1 concurrent race coverage` | new tests/phase_c_1_race.rs, all callers updated |

C.1.0 lands first, gets reviewed, then C.1.1–C.1.3 land in a fast
sequence. C.2 (`Arc<RwLock<S>> → Arc<S>`) cannot start until C.1.3
merges.

## 8. Risk register

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Missed `with_write_lock` on a write path → torn write under race | medium | C.1.3 race tests catch this within 10s; code review per access site in §3.1 |
| `unused_mut` lint fails CI on caller-side `let mut storage` | high | `rg "let mut .*storage" crates/` pre-commit; fix all 13 callers in §5 |
| Caller relied on `&mut TableData` returned by `get_table_mut` | medium | Provide closure-based replacement `with_table_mut(&self, name, \|t\| { ... })` in C.1.1; deprecate the old method |
| `current_tx_id` read races with writer | very low | `u64` reads are atomic on all supported targets (x86_64/aarch64/ARMv7+); document invariant in field comment |
| `flush_parallel` spawns threads holding `&mut tables` via `with_write_lock` | medium | The `MutexGuard` is dropped before `s.spawn`, so spawned threads re-acquire — no deadlock risk, but verify in C.1.3 race test |

## 9. References

* Parent doc: `docs/releases/v5.0.0/PHASE_C_SERVER_LOCK_REMOVAL.md`
* Trait surface: `crates/storage/src/engine.rs:916`
* Field definitions: `crates/storage/src/file_storage.rs:19`
* Mini Step analysis: `docs/releases/v4.0.0/PHASE_B_MINISTEP_READ.md`
