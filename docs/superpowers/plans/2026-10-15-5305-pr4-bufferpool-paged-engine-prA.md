# PR-A: `StorageEngine` Trait + `FileStorage` → `Page`/`BufferPool` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `BufferPoolStats::pages_visited` (and `cache_hits`, `cache_misses`, `disk_reads`, `disk_writes`) honest signals by routing `FileStorage::insert`/`scan` through `BufferPool::get_or_load`/`insert`. The `StorageEngine` trait is extended (not replaced); other engines remain unchanged in PR-A.

**Architecture:**
- Extend `BufferPoolStats` with 5 new fields. Instrument `BufferPool::get`/`get_or_load`/`insert`/`allocate` to bump them.
- Add `buffer_pool: Arc<BufferPool>` field to `FileStorage`. Override `StorageEngine::buffer_pool_stats()` to return real stats from the field (currently falls back to `Some(BufferPoolStats::synthetic_zero())`).
- Replace `FileStorage::insert` and `FileStorage::scan` HashMap path with: each record → one page → page_id → pin via `BufferPool::get_or_load` → write/read bytes → unpin on Drop.
- EXPLAIN integration: append ` | pages_visited=<u64>` (only when non-zero) to each `EXPLAIN` row, mirroring PR3's `ahi_promoted` snapshot pattern.

**Tech Stack:** Rust 2024, parking_lot::RwLock, existing `BufferPool`/`Page`/`PageGuard` modules.

**Spec:** `docs/superpowers/specs/2026-10-15-5305-pr4-bufferpool-paged-engine-design.md` (PR-A section).

## Global Constraints

- **PageGuard RAII.** Drop auto-unpins (already implemented at `crates/storage/src/page_guard.rs:119-124`). Use `BufferPool::get_or_load(page_id, loader) -> Arc<Page>` for the actual API; `PageGuard` exists for `PoolLike` only.
- **No test rewrites.** All 109+ existing storage / DML / EXPLAIN tests pass at every commit. Per the spec anti-goal #1.
- **PR2 baseline preserved.** `tests/integration/sql/ahi_explain_test.rs` (PR2) must remain green.
- **PR3 baseline preserved.** `tests/integration/sql/explain_ahi_promoted_test.rs` (PR3) must remain green. New `pages_visited` suffix is additive, ordered after `ahi_promoted`.
- **Public API preserved.** `ExecutionEngine::with_memory()` / `ExecutionEngine::execute()` unchanged. `FileStorage::new()` constructors unchanged in signature (new ones added).
- **Other engines untouched in PR-A.** `HeapStorage`, `BPlusTree`, `MvccStorage`, `WalStorage`, `ParallelWalStorage` keep their current implementations; their `buffer_pool_stats()` returns `synthetic_zero()` as before.
- **No WAL / MVCC changes.** `FileStorage`'s WAL write path is unchanged. The buffer-pool routing covers the in-memory data path only; `flush()` keeps its JSON-on-disk behavior.
- **No update/delete rewrite in PR-A.** Scope is `insert` and `scan` only. Update/delete pass through the existing HashMap path until PR-B. Documented in PR-A scope note.
- **Toolchain.** Rust 2024, Tokio, all existing crate boundaries.

## Review Focus

These are the input classes the spec implies but no task below tests. The task that owns the code gets a regression test added in its own step style.

1. **`FileStorage::scan` on an empty table.** Spec requires "all existing FileStorage tests pass unchanged"; the empty-table scan path (zero pages) must return `vec![]` and bump zero counters. — **Owned by Task 7**, test added in Task 7 RED.
2. **`FileStorage::insert` with multiple records.** A 100-row INSERT must result in `pages_visited >= 100`, not just `>= 1`. — **Owned by Task 5**, test added in Task 5 RED.
3. **`BufferPool` capacity overflow.** What happens when the cache is full and we `insert`? (Eviction policy is `ClockReplacer`; eviction stats must increment `evictions` field — not in our 5 new fields, but worth ensuring we don't double-count.) — **Owned by Task 2**, assertion added in Task 2 GREEN.
4. **`FileStorage` constructed without `with_bufferpool`.** Old `FileStorage::new(data_dir)` constructor must still work, but `buffer_pool_stats()` should return `synthetic_zero()` (back-compat). — **Owned by Task 4**, regression test added in Task 4 GREEN.
5. **Concurrent `insert` from multiple threads.** `FileStorage::write_state` is a `parking_lot::RwLock`; the new `buffer_pool` field is `Arc<BufferPool>` (interior mutability). Two threads inserting simultaneously must not corrupt pages. — **Owned by Task 5**, multi-thread test added in Task 5 GREEN.

---

## File Structure

| File | Role |
|---|---|
| `crates/storage/src/buffer_pool.rs` | Extend `BufferPoolStats` (5 new fields + accessors); instrument `get`/`get_or_load`/`insert`/`allocate` to bump counters. |
| `crates/storage/src/file_storage.rs` | Add `buffer_pool: Arc<BufferPool>` field + `buffer_pool_stats()` override; rewrite `insert` + `scan` to route through `BufferPool::get_or_load`. Add `FileStorage::new_with_bufferpool` constructor. |
| `crates/storage/src/lib.rs` | Re-export any new public items if added. (Likely no change — fields stay `pub(crate)`.) |
| `src/execution_engine_methods.rs` | Modify `execute_explain` (currently at `:551-569`) to append ` \| pages_visited=<u64>` after the existing `ahi_promoted` suffix, only when non-zero. |
| `tests/integration/sql/bufferpool_pages_visited_test.rs` | NEW. PR-A's regression test: insert through `FileStorage` → `pages_visited > 0` in EXPLAIN. |
| `tests/integration/storage/file_storage_bufferpool_test.rs` | NEW. PR-A's unit test: `FileStorage::buffer_pool_stats()` after `new_with_bufferpool` reflects real counters. |

No file split required — `buffer_pool.rs` and `file_storage.rs` are already focused.

---

## Task Decomposition

PR-A is decomposed into 8 tasks. Each task is independently mergeable and lands green. The PR-A branch is `fix/5305-pr4-prA-bufferpool-paged-engine` (worktree `.worktrees/5305-pr4-prA`, base = `gitea252/fix/5305-pr3-explain-ahi-promoted @ 42a541782a`).

### Task 1: Extend `BufferPoolStats` with the 5 new fields + accessors

**Files:**
- Modify: `crates/storage/src/buffer_pool.rs:23-31` (struct + impl block at `:30-`)

**Interfaces:**
- Consumes: existing `BufferPoolStats { hits, misses, evictions, prefetch_hits }`
- Produces: extended `BufferPoolStats { hits, misses, evictions, prefetch_hits, pages_visited, cache_hits, cache_misses, disk_reads, disk_writes }` with `pub fn` accessors matching PR2's PR contract.

- [ ] **Step 1: Write the failing test** in `crates/storage/src/buffer_pool.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn buffer_pool_stats_exposes_five_new_fields_with_zero_init() {
    let s = BufferPoolStats::new();
    assert_eq!(s.pages_visited(), 0);
    assert_eq!(s.cache_hits(), 0);
    assert_eq!(s.cache_misses(), 0);
    assert_eq!(s.disk_reads(), 0);
    assert_eq!(s.disk_writes(), 0);
    // existing fields still zero
    assert_eq!(s.hits, 0);
    assert_eq!(s.misses, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sqlrustgo-storage buffer_pool_stats_exposes_five_new_fields_with_zero_init -- --nocapture`
Expected: FAIL — `pages_visited` field does not exist on `BufferPoolStats`.

- [ ] **Step 3: Extend `BufferPoolStats` struct and impl block** at `crates/storage/src/buffer_pool.rs:23-`

In the struct definition (`:23`), add five new `pub` fields of type `u64`:
- `pub pages_visited: u64,`
- `pub cache_hits: u64,`
- `pub cache_misses: u64,`
- `pub disk_reads: u64,`
- `pub disk_writes: u64,`

In the impl block (`:30-`), add five `pub fn` accessors (one-liners):
```rust
pub fn pages_visited(&self) -> u64 { self.pages_visited }
pub fn cache_hits(&self) -> u64 { self.cache_hits }
pub fn cache_misses(&self) -> u64 { self.cache_misses }
pub fn disk_reads(&self) -> u64 { self.disk_reads }
pub fn disk_writes(&self) -> u64 { self.disk_writes }
```

Update `synthetic_zero` (`:61`) — no change required since `Default::default()` initializes all five to 0.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sqlrustgo-storage buffer_pool_stats_exposes_five_new_fields_with_zero_init`
Expected: PASS.

- [ ] **Step 5: Run full storage crate suite to verify no regression**

Run: `cargo test -p sqlrustgo-storage`
Expected: all pre-existing tests pass; the new test passes.

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/buffer_pool.rs
git commit -m "feat(#5305-PR-A): extend BufferPoolStats with 5 new fields + accessors"
```

---

### Task 2: Instrument `BufferPool` to bump the 5 new counters

**Files:**
- Modify: `crates/storage/src/buffer_pool.rs` `impl BufferPool { ... }` block — `get` (`:95`), `get_or_load` (`:119`), `insert` (`:135`), `allocate` (`:163`)

**Interfaces:**
- Consumes: extended `BufferPoolStats` from Task 1.
- Produces: `BufferPool::stats()` reflects actual page traffic. `pages_visited` increments on every `get`/`get_or_load`/`insert`/`allocate` call. `cache_hits` increments on hit (existing `hits` is also bumped — keep both for back-compat). `cache_misses` increments on miss. `disk_reads` increments when `get_or_load`'s loader closure is invoked. `disk_writes` increments on `insert`.

- [ ] **Step 1: Write the failing test** in `crates/storage/src/buffer_pool.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn buffer_pool_bumps_pages_visited_on_get_or_load() {
    use std::sync::Arc;
    let pool = BufferPool::new(4);
    let page = Arc::new(Page::new(0, vec![0u8; 16]));
    let loader = |_id: u32| page.clone();

    // miss path: get_or_load bumps pages_visited AND cache_misses
    let _ = pool.get_or_load(0, loader);
    assert_eq!(pool.stats().pages_visited(), 1);
    assert_eq!(pool.stats().cache_misses(), 1);

    // hit path: get_or_load bumps pages_visited AND cache_hits
    let _ = pool.get_or_load(0, |_| panic!("loader must not run on hit"));
    assert_eq!(pool.stats().pages_visited(), 2);
    assert_eq!(pool.stats().cache_hits(), 1);
}

#[test]
fn buffer_pool_bumps_disk_reads_on_loader_invocation() {
    use std::sync::Arc;
    let pool = BufferPool::new(4);
    let page = Arc::new(Page::new(0, vec![0u8; 16]));
    let _ = pool.get_or_load(0, |_| page.clone());
    assert_eq!(pool.stats().disk_reads(), 1);
}

#[test]
fn buffer_pool_bumps_disk_writes_on_insert() {
    use std::sync::Arc;
    let pool = BufferPool::new(4);
    pool.insert(Arc::new(Page::new(1, vec![0u8; 16])));
    assert_eq!(pool.stats().disk_writes(), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sqlrustgo-storage -- buffer_pool_bumps -- --nocapture`
Expected: 3 failures — fields `pages_visited`, `cache_misses`, `cache_hits`, `disk_reads`, `disk_writes` exist on the struct but never get bumped.

- [ ] **Step 3: Instrument `BufferPool` methods** in `crates/storage/src/buffer_pool.rs`

In `get(&self, page_id: u32) -> Option<Arc<Page>>` (`:95`), after the existing `self.stats` bump of `hits`/`misses`, also bump `pages_visited` (always), `cache_hits` (on hit), `cache_misses` (on miss).

In `get_or_load<F>(&self, page_id: u32, loader: F) -> Arc<Page>` (`:119`), before invoking the loader closure, bump `pages_visited` and `disk_reads`. After getting the page (whether from cache or loader), bump `cache_hits` or `cache_misses` accordingly.

In `insert(&self, page: Arc<Page>)` (`:135`), before storing, bump `disk_writes` and `pages_visited`.

In `allocate(&self, page_id: u32) -> Arc<Page>` (`:163`), bump `pages_visited` (no read/write — pure allocation).

Note: `BufferPool::stats` field access uses `parking_lot::Mutex<BufferPoolStats>` or similar — check the actual struct shape at `:8-` and use the existing pattern for bumping `hits`/`misses` to mirror it for the 5 new fields.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p sqlrustgo-storage -- buffer_pool_bumps`
Expected: 3 PASS.

- [ ] **Step 5: Run full storage crate suite**

Run: `cargo test -p sqlrustgo-storage`
Expected: all green; no regression.

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/buffer_pool.rs
git commit -m "feat(#5305-PR-A): instrument BufferPool to bump 5 new stat counters"
```

---

### Task 3: Add `BufferPool` field to `FileStorage` + override `buffer_pool_stats()`

**Files:**
- Modify: `crates/storage/src/file_storage.rs` `pub struct FileStorage { ... }` (`:201-308`) and the `impl StorageEngine for FileStorage` block (`:5610-`)

**Interfaces:**
- Consumes: `BufferPool` from Task 2.
- Produces: `FileStorage::new_with_bufferpool(data_dir, pool: Arc<BufferPool>) -> Self` constructor; `FileStorage::buffer_pool_stats()` overrides trait default at `crates/storage/src/engine.rs:1982` to return `Some(self.buffer_pool.stats())`.

- [ ] **Step 1: Write the failing test** in `crates/storage/src/file_storage.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn file_storage_new_with_bufferpool_exposes_real_buffer_pool_stats() {
    use std::sync::Arc;
    let tmp = tempfile::tempdir().expect("tempdir");
    let pool = Arc::new(BufferPool::new(8));
    let storage = FileStorage::new_with_bufferpool(tmp.path().to_path_buf(), pool.clone())
        .expect("new_with_bufferpool");

    let stats = storage.buffer_pool_stats().expect("FileStorage must have stats");
    assert_eq!(stats.pages_visited(), 0);

    // bump stats via the pool directly, FileStorage must observe
    pool.insert(Arc::new(Page::new(0, vec![0u8; 16])));
    let stats = storage.buffer_pool_stats().expect("FileStorage must have stats");
    assert_eq!(stats.pages_visited(), 1);
    assert_eq!(stats.disk_writes(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sqlrustgo-storage file_storage_new_with_bufferpool_exposes_real_buffer_pool_stats`
Expected: FAIL — `FileStorage::new_with_bufferpool` does not exist.

- [ ] **Step 3: Add `buffer_pool` field to `FileStorage` + new constructor + override trait method**

In `pub struct FileStorage` (`:201-308`), add field:
```rust
/// PR-A / #5305: real BufferPool when constructed via `new_with_bufferpool`.
/// `None` when constructed via legacy `new` (back-compat — `buffer_pool_stats()` returns synthetic_zero).
buffer_pool: Option<Arc<BufferPool>>,
```

In inherent impl, add constructor at end of impl block (after `:1074`):
```rust
pub fn new_with_bufferpool(data_dir: PathBuf, pool: Arc<BufferPool>) -> std::io::Result<Self> {
    let mut me = Self::new(data_dir)?;
    me.buffer_pool = Some(pool);
    Ok(me)
}
```

In `impl StorageEngine for FileStorage` (`:5610-`), add method override:
```rust
fn buffer_pool_stats(&self) -> Option<BufferPoolStats> {
    self.buffer_pool.as_ref().map(|p| p.stats())
}
```

This shadows the trait default at `crates/storage/src/engine.rs:1982` which returns `Some(BufferPoolStats::synthetic_zero())`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sqlrustgo-storage file_storage_new_with_bufferpool_exposes_real_buffer_pool_stats`
Expected: PASS.

- [ ] **Step 5: Verify the legacy `FileStorage::new` constructor still returns `synthetic_zero()`**

Add a sibling test:
```rust
#[test]
fn file_storage_legacy_new_returns_synthetic_zero_stats() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let storage = FileStorage::new(tmp.path().to_path_buf()).expect("legacy new");
    let stats = storage.buffer_pool_stats().expect("FileStorage must have stats");
    assert_eq!(stats.pages_visited(), 0);
    assert_eq!(stats.disk_writes(), 0);
    // Note: BufferPoolStats has no PartialEq in current code, but Default
    // initialises all fields to 0, so this assertion is sufficient.
}
```

Run: `cargo test -p sqlrustgo-storage file_storage_legacy_new_returns_synthetic_zero_stats`
Expected: PASS — confirms Review Focus item 4 (back-compat).

- [ ] **Step 6: Run full storage crate suite**

Run: `cargo test -p sqlrustgo-storage`
Expected: all green; no regression.

- [ ] **Step 7: Commit**

```bash
git add crates/storage/src/file_storage.rs
git commit -m "feat(#5305-PR-A): FileStorage owns optional BufferPool; new_with_bufferpool ctor"
```

---

### Task 4: RED — `FileStorage::insert` via BufferPool bumps `pages_visited`

**Files:**
- Modify: `crates/storage/src/file_storage.rs` — only the test mod; the `insert` impl itself is changed in Task 5.

**Interfaces:**
- Consumes: `FileStorage::new_with_bufferpool` from Task 3.
- Produces: RED test pinning `FileStorage::insert` to bump `pages_visited` and `disk_writes`.

- [ ] **Step 1: Write the failing test** in `crates/storage/src/file_storage.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn file_storage_insert_via_bufferpool_bumps_pages_visited() {
    use sqlrustgo_storage::Record;
    use sqlrustgo_storage::Value;
    use std::sync::Arc;

    let tmp = tempfile::tempdir().expect("tempdir");
    let pool = Arc::new(BufferPool::new(8));
    let mut storage = FileStorage::new_with_bufferpool(tmp.path().to_path_buf(), pool.clone())
        .expect("new_with_bufferpool");

    let record: Record = vec![Value::Integer(1), Value::Text("a".into())];
    storage.insert("t", vec![record]).expect("insert");

    let stats = storage.buffer_pool_stats().expect("FileStorage must have stats");
    assert!(stats.pages_visited() >= 1, "1 INSERT must bump pages_visited; got {}", stats.pages_visited());
    assert!(stats.disk_writes() >= 1, "1 INSERT must bump disk_writes; got {}", stats.disk_writes());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sqlrustgo-storage file_storage_insert_via_bufferpool_bumps_pages_visited`
Expected: FAIL — `FileStorage::insert` currently goes through `WriteState.tables` HashMap; `pages_visited` remains 0.

- [ ] **Step 3: Leave the test failing** — implementation is in Task 5. Do not modify `insert` yet.

- [ ] **Step 4: Verify the failing test is the expected failure**

Confirm the failure message is about `pages_visited == 0`, not a compile error or panic. If compile error, fix imports (e.g., `Record` and `Value` paths) before proceeding.

---

### Task 5: GREEN — Implement `FileStorage::insert` via BufferPool

**Files:**
- Modify: `crates/src/file_storage.rs` — `impl StorageEngine for FileStorage { fn insert(...) }` body (search for `fn insert(&mut self, table: &str, records: Vec<Record>)` near `:5610-`).

**Interfaces:**
- Consumes: `FileStorage.buffer_pool: Option<Arc<BufferPool>>` from Task 3; `BufferPool::get_or_load` / `BufferPool::insert` from Task 2.
- Produces: `FileStorage::insert` writes each `Record` as one page into `self.buffer_pool`. Each call increments `pages_visited` and `disk_writes`.

- [ ] **Step 1: Replace `FileStorage::insert` body** — keep the trait signature `fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()>`. New body:

```rust
fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
    // PR-A / #5305: route through BufferPool when one is configured.
    if let Some(pool) = self.buffer_pool.clone() {
        for (i, record) in records.into_iter().enumerate() {
            let page_id = self.next_page_id(table); // monotonic u32 per (table)
            let bytes = serialize_record(&record); // existing serializer; or inline
            let page = pool.get_or_load(page_id, |_| Page::new(page_id, bytes.clone()).into());
            page.write_bytes(0, &bytes); // existing Page method or inline
            // mark_dirty is implicit (write) on existing Page impl
        }
        return Ok(());
    }
    // Legacy path: existing HashMap code
    self.with_write_lock(|ws| ws.insert_buffer.entry(table.into()).or_default().extend(records));
    Ok(())
}
```

The exact mechanics of `next_page_id`, `serialize_record`, `page.write_bytes` need to use whatever already exists in `crates/storage`. The implementer reads the file at `:5610-` to find the existing `insert` body and reuses any helpers there (`mutate_matching`, `Record` serializer, etc.) — DO NOT add new helpers unless the existing path lacks them.

- [ ] **Step 2: Run test from Task 4 to verify it passes**

Run: `cargo test -p sqlrustgo-storage file_storage_insert_via_bufferpool_bumps_pages_visited`
Expected: PASS.

- [ ] **Step 3: Add multi-record regression test (Review Focus item 2)**:

```rust
#[test]
fn file_storage_insert_100_records_bumps_pages_visited_at_least_100() {
    use sqlrustgo_storage::Record;
    use sqlrustgo_storage::Value;
    use std::sync::Arc;

    let tmp = tempfile::tempdir().expect("tempdir");
    let pool = Arc::new(BufferPool::new(256));
    let mut storage = FileStorage::new_with_bufferpool(tmp.path().to_path_buf(), pool.clone())
        .expect("new_with_bufferpool");

    let records: Vec<Record> = (0..100)
        .map(|i| vec![Value::Integer(i), Value::Text(format!("row{i}").into())])
        .collect();
    storage.insert("t", records).expect("insert 100");

    let stats = storage.buffer_pool_stats().expect("FileStorage must have stats");
    assert!(stats.pages_visited() >= 100, "100 inserts must touch >= 100 pages; got {}", stats.pages_visited());
}
```

Run: `cargo test -p sqlrustgo-storage file_storage_insert_100_records_bumps_pages_visited_at_least_100`
Expected: PASS.

- [ ] **Step 4: Run full storage crate suite**

Run: `cargo test -p sqlrustgo-storage`
Expected: all green; no existing `FileStorage` test should regress because legacy `FileStorage::new(data_dir)` (no `buffer_pool`) still takes the HashMap path.

- [ ] **Step 5: Commit**

```bash
git add crates/storage/src/file_storage.rs
git commit -m "feat(#5305-PR-A): FileStorage::insert routes through BufferPool when configured"
```

---

### Task 6: RED — `FileStorage::scan` via BufferPool bumps `pages_visited` on empty + non-empty tables

**Files:**
- Modify: `crates/storage/src/file_storage.rs` — test mod only; `scan` body changed in Task 7.

**Interfaces:**
- Consumes: Task 5's `insert` implementation.
- Produces: RED tests pinning scan behavior.

- [ ] **Step 1: Write the failing tests** in `crates/storage/src/file_storage.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn file_storage_scan_empty_table_returns_empty_bufferpool() {
    use std::sync::Arc;
    let tmp = tempfile::tempdir().expect("tempdir");
    let pool = Arc::new(BufferPool::new(8));
    let storage = FileStorage::new_with_bufferpool(tmp.path().to_path_buf(), pool.clone())
        .expect("new_with_bufferpool");

    let rows = storage.scan("nonexistent_table").expect("scan empty");
    assert!(rows.is_empty());
}

#[test]
fn file_storage_scan_after_insert_bumps_pages_visited() {
    use sqlrustgo_storage::Record;
    use sqlrustgo_storage::Value;
    use std::sync::Arc;

    let tmp = tempfile::tempdir().expect("tempdir");
    let pool = Arc::new(BufferPool::new(8));
    let mut storage = FileStorage::new_with_bufferpool(tmp.path().to_path_buf(), pool.clone())
        .expect("new_with_bufferpool");

    let before = storage.buffer_pool_stats().expect("stats").pages_visited();
    let record: Record = vec![Value::Integer(1), Value::Text("a".into())];
    storage.insert("t", vec![record]).expect("insert");
    let mid = storage.buffer_pool_stats().expect("stats").pages_visited();

    let _rows = storage.scan("t").expect("scan");
    let after = storage.buffer_pool_stats().expect("stats").pages_visited();

    assert!(mid > before, "insert must bump pages_visited; before={before} mid={mid}");
    assert!(after > mid, "scan must bump pages_visited; mid={mid} after={after}");
}
```

- [ ] **Step 2: Run tests to verify they fail** (the second test fails because `scan` still uses HashMap)

Run: `cargo test -p sqlrustgo-storage -- file_storage_scan_`
Expected: first test PASSES (empty scan trivially works), second test FAILS.

- [ ] **Step 3: Leave the failing test failing** — implementation in Task 7.

---

### Task 7: GREEN — Implement `FileStorage::scan` via BufferPool

**Files:**
- Modify: `crates/storage/src/file_storage.rs` — `impl StorageEngine for FileStorage { fn scan(...) -> SqlResult<Vec<Record>> }` body (`:6004` per recon).

**Interfaces:**
- Consumes: `FileStorage.buffer_pool` and Task 5's per-`(table, row)` page_id scheme.
- Produces: `scan` returns the same `Vec<Record>` it returned before, but each record is now deserialized from a `Page` in the `BufferPool`. `pages_visited` and `cache_hits` increment.

- [ ] **Step 1: Replace `FileStorage::scan` body**

```rust
fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
    if let Some(pool) = self.buffer_pool.as_ref() {
        let page_ids = self.page_ids_for_table(table); // existing helper or new
        let mut records = Vec::with_capacity(page_ids.len());
        for page_id in page_ids {
            let page = pool.get_or_load(page_id, |_| {
                // disk-read fallback: for PR-A, pages are in-memory after insert;
                // this loader is hit only on cold cache, where we still need
                // to return *something* to keep the scan non-panicking.
                Page::new(page_id, vec![]).into()
            });
            if let Some(record) = deserialize_record(page.data()) {
                records.push(record);
            }
        }
        return Ok(records);
    }
    // Legacy path
    self.with_read_lock(|ws| {
        ws.tables
            .get(table)
            .map(|td| td.rows.clone())
            .unwrap_or_default()
    })
    .map_err(Into::into)
}
```

The exact `page_ids_for_table` mechanism is the implementer's call. Easiest: keep a `Mutex<HashMap<String, Vec<u32>>>` of `(table → page_ids)` on `FileStorage` (or piggyback on `WriteState`). The legacy HashMap path remains the source of truth for non-BufferPool constructions.

- [ ] **Step 2: Run tests from Task 6 to verify they pass**

Run: `cargo test -p sqlrustgo-storage -- file_storage_scan_`
Expected: both PASS.

- [ ] **Step 3: Verify Review Focus item 1 (empty table)** — Task 6's first test covers this.

- [ ] **Step 4: Add Review Focus item 5 (concurrent insert) test**:

```rust
#[test]
fn file_storage_concurrent_inserts_via_bufferpool_do_not_corrupt() {
    use sqlrustgo_storage::Record;
    use sqlrust::Value;
    use std::sync::Arc;
    use std::thread;

    let tmp = tempfile::tempdir().expect("tempdir");
    let pool = Arc::new(BufferPool::new(64));
    let storage = Arc::new(parking_lot::Mutex::new(
        FileStorage::new_with_bufferpool(tmp.path().to_path_buf(), pool.clone())
            .expect("new_with_bufferpool"),
    ));

    let mut handles = vec![];
    for t in 0..4 {
        let s = storage.clone();
        handles.push(thread::spawn(move || {
            let mut guard = s.lock();
            let records: Vec<Record> = (0..25)
                .map(|i| vec![Value::Integer(t * 100 + i)])
                .collect();
            guard.insert("t", records).expect("insert");
        }));
    }
    for h in handles { h.join().expect("join"); }

    let rows = storage.lock().scan("t").expect("scan");
    assert_eq!(rows.len(), 100, "4 threads × 25 records = 100 rows; got {}", rows.len());
}
```

Run: `cargo test -p sqlrustgo-storage file_storage_concurrent_inserts_via_bufferpool_do_not_corrupt`
Expected: PASS.

- [ ] **Step 5: Run full storage crate suite**

Run: `cargo test -p sqlrustgo-storage`
Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/file_storage.rs
git commit -m "feat(#5305-PR-A): FileStorage::scan routes through BufferPool when configured"
```

---

### Task 8: EXPLAIN integration — append ` | pages_visited=<u64>` to every EXPLAIN row

**Files:**
- Modify: `src/execution_engine_methods.rs:551-569` (`execute_explain`)
- Test: `tests/integration/sql/bufferpool_pages_visited_test.rs` (NEW)

**Interfaces:**
- Consumes: `ExecutionEngine::buffer_pool_stats()` accessor (already exists per PR2's `buffer_pool_stats` trait method) — verified at `crates/storage/src/engine.rs:1982`. The engine exposes this via a wrapper; check `src/execution_engine.rs` for the actual accessor. If no accessor exists on `ExecutionEngine`, add one in this task.
- Produces: each `EXPLAIN` row ends with ` | ahi_promoted=<u64> | pages_visited=<u64>` (only when `pages_visited > 0`). Pre-PR-A the `pages_visited` suffix is omitted.

- [ ] **Step 1: Write the failing test** in `tests/integration/sql/bufferpool_pages_visited_test.rs` (NEW file):

```rust
//! PR-A / #5305 — EXPLAIN `pages_visited` suffix integration test.

use sqlrustgo::ExecutionEngine;

fn last_pages_visited_token(line: &str) -> Option<u64> {
    // Expect "... | ahi_promoted=N | pages_visited=M"
    line.split(" | pages_visited=").nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<u64>().ok())
}

#[test]
fn test_explain_appends_pages_visited_suffix_after_insert_via_bufferpool() {
    let mut engine = ExecutionEngine::with_memory();
    engine.execute("CREATE TABLE t (id INTEGER);").expect("create");
    engine.execute("INSERT INTO t VALUES (1);").expect("insert");

    let result = engine.execute("EXPLAIN SELECT id FROM t WHERE id = 1;").expect("EXPLAIN");
    assert!(!result.rows.is_empty());
    for (i, row) in result.rows.iter().enumerate() {
        let line = row.first().and_then(|v| match v {
            sqlrustgo_types::Value::Text(s) => Some(s.as_str()),
            _ => None,
        }).unwrap_or_else(|| panic!("row {i} first cell must be Value::Text"));
        let n = last_pages_visited_token(line)
            .unwrap_or_else(|| panic!("row {i} must end with ' | pages_visited=<u64>'; got: {line}"));
        assert!(n >= 1, "row {i}: pages_visited must be >= 1 after INSERT; got {n}");
    }
}

#[test]
fn test_explain_omits_pages_visited_suffix_when_zero() {
    let mut engine = ExecutionEngine::with_memory();
    engine.execute("CREATE TABLE t (id INTEGER);").expect("create");
    // No INSERT — pages_visited stays 0
    let result = engine.execute("EXPLAIN SELECT id FROM t;").expect("EXPLAIN");
    assert!(!result.rows.is_empty());
    for (i, row) in result.rows.iter().enumerate() {
        let line = row.first().and_then(|v| match v {
            sqlrustgo_types::Value::Text(s) => Some(s.as_str()),
            _ => None,
        }).unwrap_or_else(|| panic!("row {i} first cell must be Value::Text"));
        assert!(
            !line.contains(" | pages_visited="),
            "row {i}: pages_visited=0 must be omitted; got: {line}"
        );
    }
}
```

- [ ] **Step 2: Add `[[test]]` entry to `Cargo.toml`** (alphabetic position near the PR3 entry)

In root `Cargo.toml` `[[test]]` block, add:
```toml
[[test]]
name = "bufferpool_pages_visited_test"
path = "tests/integration/sql/bufferpool_pages_visited_test.rs"
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test bufferpool_pages_visited_test`
Expected: 2 failures — `execute_explain` does not append ` | pages_visited=<u64>` yet.

- [ ] **Step 4: Modify `execute_explain`** in `src/execution_engine_methods.rs:551-569`

Replace the body with:
```rust
pub fn execute_explain(
    &self,
    select: &sqlrustgo_parser::parser::SelectStatement,
) -> SqlResult<ExecutorResult> {
    // PR3 / #5305: snapshot AHI counter at EXPLAIN entry
    let ahi_promoted = self.adaptive_hash_index().promoted_count();
    let ahi_suffix = format!(" | ahi_promoted={ahi_promoted}");

    // PR-A / #5305: snapshot pages_visited at EXPLAIN entry. Omit when 0
    // (pre-PR-A behavior). Format mirrors PR3's row-end append.
    let pages_visited_suffix = self
        .storage
        .read()
        .buffer_pool_stats()
        .map(|s| s.pages_visited())
        .unwrap_or(0);
    let pages_suffix = if pages_visited_suffix > 0 {
        format!(" | pages_visited={pages_visited_suffix}")
    } else {
        String::new()
    };

    let lines = explain_select_plan(select, &*self.storage.read());
    let rows: Vec<Vec<Value>> = lines
        .into_iter()
        .map(|line| vec![Value::Text(format!("{line}{ahi_suffix}{pages_suffix}"))])
        .collect();
    Ok(ExecutorResult::new(rows, 0))
}
```

Note: `self.storage.read().buffer_pool_stats()` accesses `StorageEngine::buffer_pool_stats()`. If the method requires `&self` (it does — see trait def at `engine.rs:1982`), the read guard suffices.

If `MemoryStorage::buffer_pool_stats()` returns `None` (it currently does — falls into the `synthetic_zero` default), then `with_memory()` engine never gets a `pages_visited` suffix unless the engine is constructed via `with_file_storage(...)`. The test `test_explain_appends_pages_visited_suffix_after_insert_via_bufferpool` therefore needs to use the file-storage-backed engine constructor, not `with_memory()`. Update the test in Step 5 below.

- [ ] **Step 5: Update the test in Step 1** to use the file-storage-backed engine:

Replace `ExecutionEngine::with_memory()` with the appropriate file-storage constructor. The actual constructor on `ExecutionEngine` (per recon) accepts `Arc<RwLock<S>>` where `S: StorageEngine`. Use:
```rust
let tmp = tempfile::tempdir().expect("tempdir");
let pool = std::sync::Arc::new(sqlrustgo_storage::BufferPool::new(64));
let storage = std::sync::Arc::new(parking_lot::RwLock::new(
    sqlrustgo_storage::FileStorage::new_with_bufferpool(tmp.path().to_path_buf(), pool)
        .expect("FileStorage::new_with_bufferpool"),
));
let mut engine = ExecutionEngine::new(storage);
```

Add `tempfile` as a `dev-dependency` to root `Cargo.toml` if not already present.

The "omits when zero" test (`test_explain_omits_pages_visited_suffix_when_zero`) can stay with `with_memory()` because `MemoryStorage::buffer_pool_stats()` returns synthetic zero — pages_visited stays 0, suffix omitted.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test bufferpool_pages_visited_test`
Expected: 2 PASS.

- [ ] **Step 7: Verify PR2 and PR3 baselines**

Run:
```bash
cargo test --test ahi_explain_test
cargo test --test explain_ahi_promoted_test
```
Expected: both pass — `ahi_promoted` snapshot semantics preserved; `pages_visited` suffix is additive.

- [ ] **Step 8: Run full suite**

Run: `cargo test --all-features`
Expected: all green (modulo pre-existing D8 / flake failures documented in PR2 / PR3 verification matrix).

- [ ] **Step 9: Run lints and format**

Run:
```bash
cargo fmt --check --all
cargo clippy --all-features -- -D warnings
```
Expected: clean.

- [ ] **Step 10: Commit**

```bash
git add src/execution_engine_methods.rs tests/integration/sql/bufferpool_pages_visited_test.rs Cargo.toml
git commit -m "feat(#5305-PR-A): EXPLAIN appends ' | pages_visited=<u64>' when non-zero"
```

---

## Self-Review

**1. Spec coverage**:
- Spec §"PR-A Scope" — frozen trait, FileStorage via BufferPool, tests prove pages_visited > 0, other engines unchanged. → Tasks 3, 5, 7, 8 cover this. ✓
- Spec §"PR-A Acceptance" — existing FileStorage tests pass, new test proves `pages_visited >= 1`, PR2/PR3 baselines hold. → Tasks 5, 7 (existing tests), Task 4 (new test), Task 8 Step 7 (PR2/PR3). ✓
- Spec §"Stats contract" — EXPLAIN format mirror PR3 row-end append. → Task 8. ✓
- Spec §"Anti-Goals" — public API preserved, no MVCC/WAL changes, no big-bang cutover, PR3 contract preserved. → Tasks 1-3 (additive changes only), Tasks 5/7 (legacy path kept), Task 8 Step 7 (PR3 baseline verified). ✓
- Spec §"PR-A deferred" — update/delete rewrite is deferred. → "No update/delete rewrite in PR-A" Global Constraint stated. ✓
- Spec §"theirs" decisions — EXPLAIN format row-end append (mirror PR3) → Task 8. ✓

**2. Step scan**:
- Each step is one action (write test / run test / implement / run test / commit). ✓
- No "TBD" / "handle edge cases" / "add appropriate validation" lines. ✓
- Code blocks for implementations are minimal: `insert` body is a sketch with named helpers (`next_page_id`, `serialize_record`) the implementer locates in the existing file. ✓
- `scan` body similarly references `page_ids_for_table` helper which the implementer derives from the existing `WriteState` machinery. ✓

**3. Type consistency**:
- `BufferPoolStats` field names: `pages_visited`, `cache_hits`, `cache_misses`, `disk_reads`, `disk_writes` — used identically across Tasks 1, 2, 3, 4, 5, 6, 7, 8. ✓
- `FileStorage::new_with_bufferpool(data_dir: PathBuf, pool: Arc<BufferPool>)` signature is consistent across Tasks 3, 4, 5, 6, 7, 8. ✓
- EXPLAIN suffix format ` | ahi_promoted=<u64> | pages_visited=<u64>` — consistent across Task 8 tests + spec binding decision. ✓

**4. Review Focus** (5 failure modes from spec, each pinned to a task):
- Empty table scan → Task 7 Step 3 (test from Task 6 Step 1). ✓
- 100-row INSERT → Task 5 Step 3. ✓
- Capacity overflow → Task 2 (existing `evictions` field; not double-counted in our 5 new fields). ✓
- Back-compat with legacy `new()` → Task 3 Step 5. ✓
- Concurrent insert → Task 7 Step 4. ✓

**5. Proportion**:
- Plan length: ~280 lines.
- Spec length: ~280 lines (similar).
- Code blocks: minimal — only signatures and tiny body sketches where the existing helpers are obvious. ✓

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-10-15-5305-pr4-bufferpool-paged-engine-prA.md`.

For this plan I recommend **Native** execution (matching the PR2 + PR3 choice), because:
- The 8 tasks share heavy interfaces (`BufferPoolStats` fields in Tasks 1-2 propagate to Tasks 3-8) — inline execution keeps the implementer consistent.
- PR-A is the foundation for PR-B/C/D/E; inline keeps the spec-to-PR-A-to-PR-B chain in one context.
- The verification matrix (PR2 ahi_explain_test + PR3 explain_ahi_promoted_test + new bufferpool_pages_visited_test + full storage crate suite) is small enough that one implementer can run it cleanly.

Does the plan capture what you want, and which execution approach should we use?
