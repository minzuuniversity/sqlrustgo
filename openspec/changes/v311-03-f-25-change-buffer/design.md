# V311-03 F-25 Change Buffer Design

## Context

F-25 Change Buffer is an InnoDB-style optimization: secondary index updates are **deferred** (not written to disk immediately) and **merged on read**. This reduces random I/O for high-write workloads.

**Current status**: `tests/integration/sql/change_buffer_test.rs` — ISOLATED, tests pass but zero main-path calls.

**Goal**: Integrate into `crates/storage/src/` and make it part of the actual storage engine read/write path.

## Architecture

### Change Buffer API (from ISOLATED implementation)

```rust
pub struct ChangeBuffer {
    pending: Arc<Mutex<VecDeque<ChangeEntry>>>,  // ordered queue
    by_page: Arc<Mutex<HashMap<u64, Vec<ChangeEntry>>>>, // indexed by page_id
    max_entries: usize,
    flushes: Arc<Mutex<u64>>,
    merges_on_read: Arc<Mutex<u64>>,
}

pub enum ChangeOp {
    Insert { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
    Update { key: Vec<u8>, new_value: Vec<u8> },
}

impl ChangeBuffer {
    pub fn defer_update(&self, page_id: u64, op: ChangeOp)
    pub fn merge_on_read(&self, page_id: u64) -> Vec<ChangeEntry>
    pub fn flush(&self) -> Vec<ChangeEntry>
    pub fn should_flush(&self) -> bool
    pub fn pending_count(&self) -> usize
    pub fn flush_count(&self) -> u64
    pub fn merge_count(&self) -> u64
}
```

### Integration into StorageEngine Trait

Add to `engine.rs`:

```rust
// In StorageEngine trait or a new ChangeBufferManager
pub trait ChangeBufferManager: Send + Sync {
    fn defer_update(&self, table_id: u64, page_id: u64, op: ChangeOp);
    fn merge_on_read(&self, table_id: u64, page_id: u64) -> Vec<ChangeEntry>;
    fn flush(&self, table_id: u64) -> Vec<ChangeEntry>;
    fn should_flush(&self) -> bool;
}

// StorageEngine default impl: no-op (for engines that don't use CB)
impl<T: StorageEngine> ChangeBufferManager for T {
    fn defer_update(&self, ...) {}
    fn merge_on_read(&self, ...) { vec![] }
    fn flush(&self, ...) { vec![] }
    fn should_flush(&self) -> false
}
```

### Integration into BufferPool

In `buffer_pool.rs`, modify `read_page`:

```rust
pub fn read_page(&self, page_id: u64) -> Option<PageGuard> {
    // ... existing cache lookup ...
    
    // NEW: merge change buffer entries for this page
    if let Some(cb) = &self.change_buffer {
        let entries = cb.merge_on_read(page_id);
        if !entries.is_empty() {
            // Apply entries to page before returning
            self.apply_change_entries(page, entries);
        }
    }
    
    Some(PageGuard::new(page))
}
```

### Integration into ClusteredTable (secondary index writes)

In `clustered_table.rs`, when inserting/updating rows with secondary indexes:

```rust
fn update_secondary_indexes(&self, op: ChangeOp) {
    // Instead of immediately writing to secondary index pages,
    // defer to change buffer
    if let Some(cb) = &self.change_buffer {
        for (page_id, index_entry) in self.calculate_index_pages(op) {
            cb.defer_update(page_id, index_entry);
        }
    } else {
        // Fallback: immediate write (no change buffer)
        self.write_secondary_indexImmediate(op);
    }
}
```

## File Plan

| Action | File | Notes |
|--------|------|-------|
| RENAME | `tests/integration/sql/change_buffer_test.rs` → `crates/storage/src/change_buffer.rs` | Move ISOLATED impl to main crate |
| MODIFY | `crates/storage/src/lib.rs` | Add `pub mod change_buffer;` |
| MODIFY | `crates/storage/src/change_buffer.rs` | Add `pub use tests::change_buffer_test::...` or rewrite |
| MODIFY | `crates/storage/src/engine.rs` | Add `change_buffer` field to `StorageEngine` or new trait |
| MODIFY | `crates/storage/src/buffer_pool.rs` | Add `merge_on_read()` call in `read_page()` |
| MODIFY | `crates/storage/src/clustered_table.rs` | Add `defer_update()` call in secondary index write |
| NEW | `tests/change_buffer_main_path_test.rs` | Integration test |
| NEW | `crates/storage/src/change_buffer_tests.rs` | Unit tests (moved from old location) |

## Key Decisions

### Decision 1: ChangeBuffer lives on StorageEngine, not global

**Tradeoff**: Global singleton (easier) vs per-table/engine (more flexible)
**Choice**: Per-`StorageEngine` instance via `Arc<ChangeBuffer>` field

**Rationale**: Different tables may have different index patterns. Global singleton
prevents concurrent isolation of different table's change buffers.

### Decision 2: Feature-gated with `change-buffer` feature

**Tradeoff**: Always-on (simpler) vs feature-gated (smaller binary)
**Choice**: Feature-gated behind `change-buffer` feature in Cargo.toml

**Rationale**: Not all workloads benefit from change buffer. Keep default binary small.

### Decision 3: Merge-on-read vs Merge-on-flush

**Tradeoff**: InnoDB uses merge-on-read (immediate consistency) vs merge-on-flush (faster writes)
**Choice**: Merge-on-read (matching current ISOLATED implementation)

**Rationale**: `merge_on_read()` already implemented in ISOLATED code. Simpler to integrate.

## Testing Strategy

1. **Unit tests**: Move `tests/integration/sql/change_buffer_test.rs` content to `crates/storage/src/change_buffer_tests.rs`
2. **Integration test**: `tests/change_buffer_main_path_test.rs`
   - Create table with secondary index
   - Insert rows (triggers `defer_update`)
   - Query that reads the secondary index pages (triggers `merge_on_read`)
   - Verify results are correct
3. **Crash recovery test**:
   - Insert rows
   - Simulate crash (no checkpoint)
   - Restart
   - Verify deferred updates are recovered (or replayed from WAL)

## Non-Goals

- DWB (Double-Write Buffer) integration — separate V311-04
- Background merge thread — v3.12+ scope
- Multi-page flush optimization — v3.12+ scope
- Adaptive change buffer sizing — v3.12+ scope
