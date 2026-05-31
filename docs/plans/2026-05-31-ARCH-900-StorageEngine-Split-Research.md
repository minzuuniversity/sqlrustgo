# ARCH-900 StorageEngine Trait Split Research Report

**Date:** 2026-05-31  
**Author:** SQLRustGo Architecture Researcher  
**Status:** Research Complete  
**Branch:** develop/v3.8.0

---

## 1. Executive Summary

This report analyzes the feasibility of splitting the monolithic `StorageEngine` trait into three focused interfaces:
- **ReadEngine** — scan, query, and read operations
- **WriteEngine** — insert, update, delete, and transaction operations  
- **RecoveryEngine** — WAL replay and crash recovery

The current `StorageEngine` trait contains **18 methods** with mixed concerns. Analysis shows a clean separation is achievable with moderate effort.

---

## 2. Current StorageEngine Analysis

### 2.1 StorageEngine Trait Location
- **Definition:** `crates/storage/src/engine.rs` (lines 449-517)
- **Implementations:**
  - `MemoryStorage` — in `engine.rs` (lines 544-738)
  - `FileStorage` — in `file_storage.rs` (lines 1262+)
  - `ColumnarStorage` — in `columnar/storage.rs` (lines 479+)
  - `WalStorage<S>` — wrapper in `wal_storage.rs` (lines 6-943)

### 2.2 Complete Method List with Classification

| Method | Signature | Category | Proposed Split |
|--------|-----------|----------|----------------|
| `scan` | `fn scan(&self, table: &str) -> SqlResult<Vec<Record>>` | **Read** | ReadEngine |
| `get_table_info` | `fn get_table_info(&self, table: &str) -> SqlResult<TableInfo>` | **Read** | ReadEngine |
| `has_table` | `fn has_table(&self, table: &str) -> bool` | **Read** | ReadEngine |
| `list_tables` | `fn list_tables(&self) -> Vec<String>` | **Read** | ReadEngine |
| `get_trigger` | `fn get_trigger(&self, name: &str) -> Option<TriggerInfo>` | **Read** | ReadEngine |
| `list_triggers` | `fn list_triggers(&self, table: &str) -> Vec<TriggerInfo>` | **Read** | ReadEngine |
| `list_indexes` | `fn list_indexes(&self, table: &str) -> Vec<(String, String)>` | **Read** | ReadEngine |
| `has_view` | `fn has_view(&self, name: &str) -> bool` | **Read** | ReadEngine |
| `insert` | `fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()>` | **Write** | WriteEngine |
| `delete` | `fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize>` | **Write** | WriteEngine |
| `delete_if` | `fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize>` | **Write** | WriteEngine |
| `update` | `fn update(&mut self, table: &str, _filters: &[Value], _updates: &[(usize, Value)]) -> SqlResult<usize>` | **Write** | WriteEngine |
| `update_if` | `fn update_if(&mut self, table: &str, filter: &RowFilter, mutation: &RowMutation) -> SqlResult<usize>` | **Write** | WriteEngine |
| `create_table` | `fn create_table(&mut self, info: &TableInfo) -> SqlResult<()>` | **Write** | WriteEngine |
| `drop_table` | `fn drop_table(&mut self, table: &str) -> SqlResult<()>` | **Write** | WriteEngine |
| `add_column` | `fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()>` | **Write** | WriteEngine |
| `rename_table` | `fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()>` | **Write** | WriteEngine |
| `create_index` | `fn create_index(&mut self, table: &str, column: &str, column_index: usize) -> SqlResult<()>` | **Write** | WriteEngine |
| `drop_index` | `fn drop_index(&mut self, table: &str, column: &str) -> SqlResult<()>` | **Write** | WriteEngine |
| `create_trigger` | `fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()>` | **Write** | WriteEngine |
| `drop_trigger` | `fn drop_trigger(&mut self, name: &str) -> SqlResult<()>` | **Write** | WriteEngine |

### 2.3 Method Count Summary

| Category | Count | Percentage |
|----------|-------|------------|
| Read operations | 8 | 40% |
| Write operations | 12 | 60% |
| **Total** | **20** | 100% |

---

## 3. Proposed Trait Split Design

### 3.1 ReadEngine Trait

```rust
/// Read-only operations trait
pub trait ReadEngine: Send + Sync {
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>>;
    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo>;
    fn has_table(&self, table: &str) -> bool;
    fn list_tables(&self) -> Vec<String>;
    fn get_trigger(&self, name: &str) -> Option<TriggerInfo>;
    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo>;
    fn list_indexes(&self, table: &str) -> Vec<(String, String)>;
    fn has_view(&self, name: &str) -> bool;
}
```

### 3.2 WriteEngine Trait

```rust
/// Write operations trait
pub trait WriteEngine: Send + Sync {
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()>;
    fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize>;
    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize>;
    fn update(&mut self, table: &str, _filters: &[Value], _updates: &[(usize, Value)]) -> SqlResult<usize>;
    fn update_if(&mut self, table: &str, filter: &RowFilter, mutation: &RowMutation) -> SqlResult<usize>;
    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()>;
    fn drop_table(&mut self, table: &str) -> SqlResult<()>;
    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()>;
    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()>;
    fn create_index(&mut self, table: &str, column: &str, column_index: usize) -> SqlResult<()>;
    fn drop_index(&mut self, table: &str, column: &str) -> SqlResult<()>;
    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()>;
    fn drop_trigger(&mut self, name: &str) -> SqlResult<()>;
}
```

### 3.3 RecoveryEngine Trait

```rust
/// WAL and recovery operations trait
pub trait RecoveryEngine: Send + Sync {
    fn recover(&self) -> SqlResult<Vec<WalEntry>>;
    fn replay_entry(&mut self, entry: &WalEntry) -> SqlResult<()>;
    fn checkpoint(&self, tx_id: u64) -> SqlResult<u64>;
}
```

### 3.4 Composite StorageEngine (Backward Compatibility)

```rust
/// Composite trait combining all storage operations
/// Provides backward compatibility via inheritance
pub trait StorageEngine: ReadEngine + WriteEngine {
    // No additional methods — just composes the sub-traits
}

// Alternative: Object-safe composition pattern
pub trait StorageEngine: Send + Sync {
    fn as_read(&self) -> &dyn ReadEngine;
    fn as_write(&mut self) -> &mut dyn WriteEngine;
    fn as_recovery(&self) -> &dyn RecoveryEngine;
}
```

---

## 4. Impact Analysis

### 4.1 Crates Using StorageEngine

| Crate | Usage Pattern | Impact Level |
|-------|---------------|--------------|
| `crates/storage` | Core definitions | **Critical** |
| `crates/executor` | Query execution harness | **High** |
| `crates/optimizer` | Stats collection, MockStorage | **High** |
| `crates/planner` | Query planning | **High** |
| `crates/mysql-server` | Server implementation | **Medium** |
| `crates/server` | Connection pool, endpoints | **Medium** |
| `crates/gmp` | Audit, compliance, reports | **Medium** |
| `crates/rag` | RAG integration | **Low** |
| `crates/bench-cli` | Benchmark tooling | **Low** |
| `benches/*` | Performance benchmarks | **Low** |

### 4.2 Files Requiring Modification

| File | Changes Required |
|------|------------------|
| `crates/storage/src/engine.rs` | Split trait definition, add composed StorageEngine |
| `crates/storage/src/lib.rs` | Re-export new traits |
| `crates/storage/src/file_storage.rs` | Update impl blocks |
| `crates/storage/src/columnar/storage.rs` | Update impl blocks |
| `crates/storage/src/wal_storage.rs` | Update impl + RecoveryEngine methods |
| `crates/executor/src/harness.rs` | May need trait bounds update |
| `crates/optimizer/src/stats_collector.rs` | MockStorage update |
| `crates/optimizer/src/stats_provider.rs` | Trait bound update |
| `crates/storage/src/vtu_guard.rs` | MockStorage update |
| `crates/gmp/src/compliance.rs` | Test storage creation |
| `crates/gmp/src/report.rs` | Test storage creation |

### 4.3 Dependencies Between Traits

```
StorageEngine
    ├── ReadEngine (8 methods)
    ├── WriteEngine (12 methods)
    └── RecoveryEngine (3 methods) — operates on WAL entries, not table data
```

**Key Observation:** `RecoveryEngine` operates on a different abstraction level (WAL entries) and is currently implemented by `WalStorage<S>` which wraps another `StorageEngine`. This makes it naturally separable.

---

## 5. Split Strategy Comparison

### 5.1 Strategy A: Incremental Split (Recommended)

**Approach:**
1. Create new traits alongside existing `StorageEngine`
2. Make `StorageEngine` a composition of sub-traits
3. Update implementations one at a time
4. Maintain full backward compatibility throughout

**Pros:**
- No breaking changes to consumers
- Can be done incrementally across sprints
- Easy to rollback if issues arise
- Parallel work possible on different implementations

**Cons:**
- Longer total timeline (4-6 weeks)
- More total file changes
- Requires discipline to not break compat layer

**Implementation Order:**
1. Define `ReadEngine` + `WriteEngine` traits (week 1)
2. Create composed `StorageEngine` alias (week 1)
3. Update `MemoryStorage` (week 2)
4. Update `FileStorage` (week 3)
5. Update `ColumnarStorage` (week 3)
6. Update `WalStorage` with `RecoveryEngine` (week 4)
7. Update mock implementations (week 4)
8. Deprecate direct `StorageEngine` impls (week 5)

### 5.2 Strategy B: Aggressive Split

**Approach:**
1. Define all three traits immediately
2. Remove `StorageEngine` trait entirely
3. Update all consumers to use new traits directly
4. Use feature flags for migration period

**Pros:**
- Clean break, no legacy debt
- Forces all consumers to update
- Simpler code in long run
- Full trait specialization possible

**Cons:**
- Large PR with many moving parts
- High risk of introducing bugs
- Blocks other development
- Requires coordinated migration across teams
- Difficult to rollback

**Implementation Order:**
1. Define all three traits (day 1-2)
2. Remove `StorageEngine`, update lib.rs (day 3)
3. Bulk update all implementations (day 4-10)
4. Bulk update all consumers (day 11-20)
5. Fix compilation errors and tests (day 21-30)

### 5.3 Comparison Matrix

| Criteria | Incremental (A) | Aggressive (B) |
|----------|-----------------|----------------|
| Timeline | 4-6 weeks | 4-6 weeks |
| Risk | Low | High |
| Code churn | Distributed | Concentrated |
| Backward compat | Full | Breaking |
| Parallel work | Yes | No |
| Rollback ease | Easy | Hard |
| Team coordination | Minimal | Critical |

**Recommendation:** Strategy A (Incremental) is preferred for a production system with active development.

---

## 6. Detailed Work Estimation

### 6.1 Task Breakdown

| Task | Files | Effort | Complexity |
|------|-------|--------|------------|
| Define ReadEngine trait | 1 | 0.5 day | Low |
| Define WriteEngine trait | 1 | 0.5 day | Low |
| Define RecoveryEngine trait | 1 | 0.5 day | Low |
| Create StorageEngine composition | 1 | 0.5 day | Low |
| Update lib.rs exports | 1 | 0.5 day | Low |
| Update MemoryStorage impl | 1 | 2 days | Medium |
| Update FileStorage impl | 1 | 3 days | High |
| Update ColumnarStorage impl | 1 | 3 days | High |
| Update WalStorage impl | 2 | 3 days | High |
| Update MockStorage (optimizer) | 1 | 1 day | Low |
| Update MockStorage (vtu_guard) | 1 | 0.5 day | Low |
| Update executor harness | 1 | 1 day | Medium |
| Update planner | 1 | 0.5 day | Low |
| Update gmp tests | 2 | 1 day | Medium |
| Update benchmarks | 5 | 2 days | Medium |
| Test and integration | - | 3 days | High |
| **Total** | **~25 files** | **~22 days** | - |

### 6.2 Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Trait design changes mid-way | Medium | High | Finalize traits before impl work |
| Consumer API breakage | High | High | Maintain compat layer throughout |
| Performance regression | Low | High | Benchmark before/after |
| Test failures in downstream | Medium | Medium | Comprehensive test coverage |
| Circular dependency issues | Low | High | Careful dependency analysis |

---

## 7. WalStorage Special Considerations

### 7.1 Current Architecture

```
WalStorage<S: StorageEngine>
    ├── inner: S (wrapped storage engine)
    ├── wal: WalManager
    └── current_tx_id: u64
```

### 7.2 WAL-Related Methods in StorageEngine

Currently, `StorageEngine` does NOT include WAL methods. `WalStorage` provides:
- `begin_transaction()`
- `commit_transaction()`
- `rollback_transaction()`
- `recover()` → returns `Vec<WalEntry>`
- `in_transaction()`
- `current_tx_id()`

### 7.3 Proposed RecoveryEngine Integration

```rust
pub struct WalStorage<S: StorageEngine + WriteEngine> {
    inner: S,
    wal: WalManager,
    recovery: RecoveryHandle, // New
    current_tx_id: u64,
}

// RecoveryEngine implementation for WalStorage
impl<S: StorageEngine + WriteEngine> RecoveryEngine for WalStorage<S> {
    fn recover(&self) -> SqlResult<Vec<WalEntry>> {
        self.wal.recover()
    }
    
    fn replay_entry(&mut self, entry: &WalEntry) -> SqlResult<()> {
        match entry.entry_type {
            WalEntryType::Insert => self.inner.insert(...),
            WalEntryType::Update => self.inner.update(...),
            WalEntryType::Delete => self.inner.delete(...),
            _ => Ok(())
        }
    }
}
```

---

## 8. Key Design Decisions

### 8.1 Object Safety vs Trait Inheritance

**Option 1: Trait Inheritance (Recommended)**
```rust
pub trait StorageEngine: ReadEngine + WriteEngine {}
```
- Simple, idiomatic Rust
- Compiler enforces completeness
- Can add default implementations

**Option 2: Delegation Pattern**
```rust
pub trait StorageEngine {
    fn read(&self) -> &dyn ReadEngine;
    fn write(&mut self) -> &mut dyn WriteEngine;
}
```
- Maximum flexibility
- Allows different implementations per operation
- More boilerplate

### 8.2 Default Implementations

Consider providing default no-op implementations for `RecoveryEngine`:
```rust
impl<S: StorageEngine> RecoveryEngine for S {
    default fn recover(&self) -> SqlResult<Vec<WalEntry>> {
        Ok(Vec::new())
    }
    default fn replay_entry(&mut self, _: &WalEntry) -> SqlResult<()> {
        Ok(())
    }
}
```

This allows implementations to opt-in to recovery features incrementally.

---

## 9. Migration Path

### Phase 1: Foundation (Week 1)
- [ ] Create `ReadEngine` trait in `engine.rs`
- [ ] Create `WriteEngine` trait in `engine.rs`
- [ ] Create `RecoveryEngine` trait in `engine.rs`
- [ ] Define `StorageEngine = ReadEngine + WriteEngine`
- [ ] Update `lib.rs` exports

### Phase 2: Core Storage (Week 2-3)
- [ ] Update `MemoryStorage` to impl new traits
- [ ] Update `FileStorage` to impl new traits
- [ ] Update `ColumnarStorage` to impl new traits

### Phase 3: WAL and Integration (Week 4)
- [ ] Add `RecoveryEngine` to `WalStorage`
- [ ] Update mock implementations
- [ ] Update executor harness

### Phase 4: Validation (Week 5-6)
- [ ] Run full test suite
- [ ] Performance benchmarking
- [ ] Update documentation
- [ ] Deprecate direct `StorageEngine` impl warnings

---

## 10. Recommendations

1. **Proceed with Strategy A (Incremental Split)** — Lower risk with same timeline
2. **Use trait inheritance** for `StorageEngine` composition
3. **Add default implementations** for `RecoveryEngine` to ease adoption
4. **Maintain the wrapper pattern** for `WalStorage` — it correctly encapsulates transaction concerns
5. **Do not rush the design phase** — trait signatures are hard to change once consumers adopt them
6. **Add deprecation warnings** early to encourage migration without forcing it
7. **Coordinate with optimizer work** — `MockStorage` updates can be batched with optimizer refactoring

---

## 11. Appendix: Current StorageEngine Full Definition

```rust
pub trait StorageEngine: Send + Sync {
    // Read operations
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>>;
    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo>;
    fn has_table(&self, table: &str) -> bool;
    fn list_tables(&self) -> Vec<String>;
    fn get_trigger(&self, name: &str) -> Option<TriggerInfo>;
    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo>;
    fn list_indexes(&self, table: &str) -> Vec<(String, String)>;
    fn has_view(&self, name: &str) -> bool;

    // Write operations
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()>;
    fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize>;
    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize>;
    fn update(&mut self, table: &str, _filters: &[Value], _updates: &[(usize, Value)]) -> SqlResult<usize>;
    fn update_if(&mut self, table: &str, filter: &RowFilter, mutation: &RowMutation) -> SqlResult<usize>;
    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()>;
    fn drop_table(&mut self, table: &str) -> SqlResult<()>;
    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()>;
    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()>;
    fn create_index(&mut self, table: &str, column: &str, column_index: usize) -> SqlResult<()>;
    fn drop_index(&mut self, table: &str, column: &str) -> SqlResult<()>;
    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()>;
    fn drop_trigger(&mut self, name: &str) -> SqlResult<()>;
}
```

---

*End of Report*
