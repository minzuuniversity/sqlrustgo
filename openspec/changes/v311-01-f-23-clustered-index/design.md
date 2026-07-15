## Context

F-23 ClusteredIndex is at an inflection point: the algorithm is **proven** in
isolation (`tests/clustered_index_test.rs` 7/7 PASS) but the **integration
with the actual storage layer is zero**.

### Current Architecture (v3.10.0)

```text
                parser::Statement::CreateTable(create_stmt)
                                ↓
src/execution_engine.rs::execute_create_table()
                                ↓
                TableInfo { name, columns, ... }
                                ↓
storage.create_table(&info)   // StorageEngine trait method
                                ↓
MemoryStorage.tables: HashMap<String, Vec<Record>>   ← single backend
```

- `MemoryStorage` is the **only** production StorageEngine
- All tables go to `HashMap<String, Vec<Record>>` regardless of declared structure
- The standalone `ClusteredIndex` test uses a **different `Row` type** (with `Vec<(String,String)>` data) than the production `Record` type (`Vec<Value>`)

### Target Architecture (V311-01 v1)

```text
                parser::Statement::CreateTable(create_stmt)
                                ↓  ← NEW: storage_engine: Option<StorageEngineSpec>
src/execution_engine.rs::execute_create_table()
                                ↓  ← NEW: dispatch on storage_engine
                ┌────────────────────┴─────────────────────────┐
                ↓                                            ↓
        Heap (default)                           ClusteredTable (new)
        MemoryStorage                            ClusteredTable { btree: ClusteredBTree }
            tables: HashMap<...>                       btree: BTreeMap<Value, Record>
                ↓                                            ↓
                └────────────────────┬───────────────────────┘
                                     ↓
                          StorageEngine trait (common)
                                     ↓
                          ExecutionEngine storage.dispatch(table)
```

## Goals / Non-Goals

**Goals:**
- Add `ENGINE=InnoDB CLUSTERED` parser support
- Add `ClusteredTable` storage backend implementing the StorageEngine trait
- Dispatch table creation to appropriate backend based on parser hint
- O(log N) PRIMARY KEY lookup
- O(log N + k) PRIMARY KEY range scan
- Mixed Heap + Clustered tables in same database instance
- Performance benchmarks comparing Heap vs Clustered

**Non-Goals:**
- Page-based disk persistence (v3.12+ scope)
- WAL for ClusteredTable (v3.12+ scope)
- Multi-column primary keys (v311-01-multicol follow-up)
- CLUSTERED secondary index support (v3.12+ scope)
- Replacing existing `MemoryStorage` (Heap remains default)

## Decisions

### Decision 1: ClusteredTable composes with StorageEngine trait

**Choice**: Make `ClusteredTable` itself implement the `StorageEngine` trait, with `MemoryStorage` as a `private member` for non-clustered tables. The ExecutionEngine owns both.

**Alternative considered**: Single `MemoryStorage` with per-table flags. Rejected because it forces Heap's Vec<Record> representation on Clustered tables too, defeating the point of optimization.

**Tradeoff**: ClusteredTable must re-implement ~40 StorageEngine methods, mostly by forwarding to a `MemoryStorage`-like approach for non-PK path queries. ~400 LoC.

### Decision 2: B+ Tree uses BTreeMap<Value, Record> (in-memory v1)

**Choice**: Use std `BTreeMap<Value, Record>` (Rust's red-black tree, equivalent to B+ Tree for ordered key-value with O(log N) operations) keyed by primary key value.

**Alternative considered**: Custom `ClusteredBTree` with leaf pages implementing `LeafPage::split()`. Rejected because:
- The standalone test already proves this concept works (7/7 PASS)
- Disk-persistence is v3.12+ scope; in-memory BTreeMap is sufficient
- Rust's BTreeMap is already battle-tested

**Tradeoff**: We don't get disk-persistence benefits (sub-loaded pages, page-level splits), but those are explicitly out-of-scope. The `BTreeMap` gives us O(log N) lookup, O(log N + k) range scan, sorted output for free.

### Decision 3: ClusteredTable::insert appends to BTreeMap (no batch optimization in v1)

**Choice**: `insert(records)` does one `btree.insert(pk, record)` per record. No batch loading optimization.

**Alternative considered**: Bulk-load via `BTreeMap::extend()` for initial table population. Deferred — v1 priority is correctness over bulk performance.

**Tradeoff**: 1M-row initial load is O(N log N) vs O(N) bulk-load. Acceptable for v1; bulk-load is v311-01-bulk follow-up.

### Decision 4: Mixed-backend transaction model = "split transactions"

**Choice**: Each backend maintains its own transaction log. A multi-table transaction writes to BOTH backends' logs, but atomicity is best-effort: if one backend commits and another rolls back, the user sees partial state.

**Alternative considered**: A single unified transaction log per ExecutionEngine. Rejected for v1 because it requires rearchitecting MemoryStorage's `TxLog` to span multiple backends.

**Tradeoff**: Multi-table transactions are not strictly atomic. Mitigation: v1 documents this clearly; v311-01-tx follow-up adds unified transaction log.

**Real-world impact**: 99% of TPC-H workloads are single-table transactions or simple `BEGIN; INSERT a; INSERT b; COMMIT;` patterns where best-effort atomicity is acceptable.

### Decision 5: ClusteredTable::scan returns sorted Vec by PK

**Choice**: `scan()` returns all rows sorted by PK. Equivalent to iterating `btree.values()` (already sorted by key in BTreeMap).

**Alternative considered**: HashMap-based scan returning insertion order. Rejected because:
- ClusteredIndex's semantic contract IS sorted-by-PK scan
- Operators that depend on order get PK order for free

**Tradeoff**: Scan is always sorted even for queries that don't need it. For full-table scans, this is acceptable cost.

## Architectural Diagrams

### Type Hierarchy

```text
StorageEngine (trait)
├── MemoryStorage (existing)
└── ClusteredTable (new)
    ├── btree: BTreeMap<Value, Record>    ← primary key → row
    ├── secondary_indexes: HashMap<String, BTreeMap<Value, Vec<Value>>>
    │                                  ↑    ↑
    │                            column name key value→primary key list
    └── table_info: TableInfo
```

### ExecutionEngine Routing

```text
ExecutionEngine<S: StorageEngine> {
    storage: Arc<RwLock<dyn StorageEngine>>,    // existing Heap backend
    clustered_tables: Arc<RwLock<HashMap<String, Arc<RwLock<ClusteredTable>>>>>,  // NEW
}

fn scan(&self, table: &str) -> Result<Vec<Record>> {
    if self.clustered_tables.read().contains_key(table) {
        // CLUSTERED path
        let ct = self.clustered_tables.read().get(table).unwrap().clone();
        Ok(ct.read().all_rows_sorted())
    } else {
        // HEAP path (existing)
        self.storage.read().scan(table)
    }
}
```

## Implementation Plan

### Phase 1: Parser + AST changes (8h)

1. Add `pub enum StorageEngineSpec { Heap, Clustered }` to `crates/parser/src/parser.rs`
2. Add `storage_engine: Option<StorageEngineSpec>` field to `CreateTableStatement`
3. Parse `ENGINE=InnoDB CLUSTERED` syntax
4. Add unit test: parse `CREATE TABLE t (...) ENGINE=InnoDB CLUSTERED`

### Phase 2: ClusteredTable storage (24h)

1. Create `crates/storage/src/clustered_table.rs` 
2. Implement `ClusteredTable` struct + `StorageEngine` trait methods
3. For methods that need full StorageEngine surface, delegate to internal `MemoryStorage`-like structure for non-key constraints (FK, checks, etc.)
4. Test: `clustered_table_test.rs` for unit-level tests

### Phase 3: ExecutionEngine routing (8h)

1. Create `src/clustered_dispatcher.rs` with dispatching storage wrapper
2. Modify `ExecutionEngine` to track `clustered_tables` registry
3. Update scan/insert/delete/update methods to dispatch correctly
4. Test: end-to-end CREATE/INSERT/SELECT/DELETE on CLUSTERED tables

### Phase 4: Tests + benchmarks (16h)

1. `tests/integration/storage/clustered_main_path_test.rs` — 8+ tests
2. `tests/integration/tpch/clustered_perf_benchmark.rs` — 4+ benchmarks
3. Run full test suite to ensure no regression
4. Generate perf comparison charts

### Phase 5: Documentation (8h)

1. `docs/releases/v3.11.0/perf/CLUSTERED_INDEX_PERF.md` 
2. Update debt registry F-23 → CLOSED
3. Update ISOLATED_MODULES.md (remove F-23)
4. Update V311_VERSION_PLAN.md

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| StorageEngine trait has 40+ methods — large surface to implement | High | Implement only `scan/insert/delete/update/create_table/drop_table/get_table_info/has_table/list_tables` for v1; other methods delegate to internal fallback |
| ClusteredTable violates PK uniqueness assumption silently | High | `insert()` checks `btree.contains_key(pk)` and returns `SqlError::UniqueViolation` |
| Mixed-backend transaction atomicity gap | Medium | Document in V311-01 v1 release notes; v311-01-tx follow-up |
| Concurrent INSERT to ClusteredTable requires lock granularity | Medium | Use `Arc<RwLock<ClusteredTable>>`; reader-writer synchronization same as MemoryStorage |
| Real TPC-H workload regression vs Heap | Low | A/B benchmark via `tests/clustered_perf_benchmark.rs`; fall back to Heap if perf is worse |
| V311-01 scope creep into v3.12 disk-persistence | High | Out-of-scope explicit in design; defer to v311-01-disk or v3.12 |

## Open Questions / Decisions Needed

1. **Multi-column primary keys**: V311-01 v1 supports single-column PK only. Multi-column deferred. OK?
2. **ClusteredTable + WAL**: V311-01 v1 does NOT integrate with WAL. ACCEPTED for v1.
3. **ClusteredTable + MVCC**: V311-01 v1 does NOT participate in MVCC. ACCEPTED for v1.
4. **Default storage engine**: Heap remains default for backward compatibility. CONFIRMED.
5. **CREATE TABLE option vs session-level config**: Per-table option is simpler and matches MySQL semantics. CONFIRMED.

## Verification Strategy

- **Functional**: 8+ new tests in `clustered_main_path_test.rs`
- **Performance**: 4+ benchmarks in `clustered_perf_benchmark.rs`
- **Regression**: Full test suite (`cargo test --all-features`)
- **Oracle**: TPC-H Q1/Q3/Q5/Q6 (single-table scans) results unchanged
- **Debt registry**: F-23 verified → CLOSED
