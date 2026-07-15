## Why

F-23 ClusteredIndex is the **#1 critical functional gap** in sqlrustgo's v3.11.0 release. The current state:

| Aspect | Status |
|--------|--------|
| Standalone test | 7/7 PASS (test-only, BTreeMap-based, ISOLATED) |
| Production integration | 0% — `crates/storage/src/bplus_tree/` only stores `<i64, u32>` value→page mappings, NOT actual rows |
| CREATE TABLE syntax | No `ENGINE=InnoDB CLUSTERED` option |
| B+ Tree page-based | NOT implemented — current index is in-memory only |
| Row storage in leaves | NOT implemented — rows still go to `MemoryStorage.tables` HashMap<String, Vec<Record>> |

The cluster index test (`tests/integration/sql/clustered_index_test.rs`) proves the algorithm works **in isolation** but cannot touch the production code path. 

**Original constraint**: `ISOLATED_MODULES.md §1 (F-23)` deprioritized integration to "v3.11.0+" pending "real disk-based integration". 

**V311-01 unblocks**:
1. **F-22 InnoDB feature parity** (current gap: 11/15 features)
2. **Hot UPDATE performance**: PRIMARY KEY lookups currently scan Vec<Record> for `WHERE id = ?` (O(N))
3. **Wide table scans**: range scans on PK become O(log N) instead of O(N)
4. **Disk persistence**: page-based storage is the v3.12 plan target — ClusteredIndex is the foundation

## What Changes

### 1. Parse `ENGINE=InnoDB CLUSTERED` syntax

- **Parser** (`crates/parser/src/parser.rs:597`): Add `storage_engine: Option<StorageEngineSpec>` field to `CreateTableStatement`
- **AST addition**: `pub enum StorageEngineSpec { Heap, Clustered }`
- **Grammar**: `CREATE TABLE t (id INT PRIMARY KEY, ...) ENGINE=InnoDB CLUSTERED` (parse with ENGINE keyword + CLUSTERED option as enum tag)
- **Default**: Heap (preserves existing behavior)

### 2. New `ClusteredTable` storage backend

- **New file**: `crates/storage/src/clustered_table.rs`
- **Type**: `pub struct ClusteredTable<S: StorageEngine>` wraps an underlying engine for rows
- **Storage**: `BTreeMap<Value, Record>` keyed by primary key value
- **API**: implements the same `StorageEngine` trait subset as `MemoryStorage`
- **Schema**: Reuses existing `TableInfo`, requires PRIMARY KEY to be declared
- **Performance**: O(log N) `get_by_pk`, O(log N) `range_scan(start, end)`, O(N) full scan (sorted)

### 3. Multi-backend dispatch in `ExecutionEngine`

- **New file**: `src/clustered_dispatcher.rs` 
- **Data model**: `ExecutionEngine.storage: Arc<RwLock<dyn StorageEngine>>` plus `clustered_tables: Arc<RwLock<HashMap<String, Arc<RwLock<ClusteredTable>>>>>` for tables opted into ClusteredIndex
- **Routing**: `scan/insert/delete/update` checks if table is clustered, dispatches accordingly
- **Backward compatibility**: All existing tests continue passing

### 4. Page-level integration (foundation for v3.12 disk persistence)

- **New file**: `crates/storage/src/bplus_tree/clustered_btree.rs`
- Uses same leaf-page model as existing test, but with `Vec<Value>` data (Record-shaped)
- For V311-01 v1: keep in-memory, defer real disk pages to v3.12
- Splits, scans, range queries, all in-memory

### 5. Wire protocol + tests

- **New test file**: `tests/integration/storage/clustered_main_path_test.rs`
  - Tests INSERT/SELECT/UPDATE/DELETE on a CLUSTERED table
  - Tests `WHERE pk = ?` (O(log N) lookup)
  - Tests `WHERE pk BETWEEN ? AND ?` (range scan)
  - Tests mixed: Heap + Clustered tables in same database
- **New test file**: `tests/integration/tpch/clustered_perf_benchmark.rs`
  - Benchmark: 1M+ row INSERT throughput (ClusteredIndex vs Heap)
  - Benchmark: 100K+ PK lookup latency (ClusteredIndex vs Heap)

### 6. Documentation

- **New**: `docs/releases/v3.11.0/perf/CLUSTERED_INDEX_PERF.md` — performance comparison
- **Update**: `docs/governance/debt/debt-registry.yaml` — F-23 VERIFIED → **CLOSED**
- **Update**: `ISOLATED_MODULES.md` — remove F-23 from §1 isolated list
- **Update**: `openspec/changes/V311_VERSION_PLAN.md` — V311-01 marked ✅ DONE

## Capabilities

### New Capabilities

- `clustered-index-storage`: tables opt into clustered primary key storage via `ENGINE=InnoDB CLUSTERED`
- `clustered-pk-lookup`: O(log N) primary-key equality scan via B+ Tree leaves
- `clustered-pk-range-scan`: O(log N + k) range scan via B+ Tree in-order iteration
- `mixed-table-engine`: a single database can contain both Heap and Clustered tables

## Impact

### Affected Files (created/modified)

| File | Type | Lines |
|------|------|-------|
| `crates/parser/src/parser.rs` | modified | +60 (storage_engine field + grammar) |
| `crates/storage/src/clustered_table.rs` | new | ~400 (table impl) |
| `crates/storage/src/bplus_tree/clustered_btree.rs` | new | ~350 (B+ tree with row storage) |
| `crates/storage/src/lib.rs` | modified | +5 (export new mod) |
| `src/clustered_dispatcher.rs` | new | ~150 (storage routing) |
| `src/execution_engine.rs` | modified | +30 (route to ClusteredTable when applicable) |
| `tests/integration/storage/clustered_main_path_test.rs` | new | ~250 |
| `tests/integration/tpch/clustered_perf_benchmark.rs` | new | ~200 |
| `docs/releases/v3.11.0/perf/CLUSTERED_INDEX_PERF.md` | new | ~150 |
| `docs/governance/debt/debt-registry.yaml` | modified | F-23 → CLOSED |

### No Breaking Changes

- Default storage engine stays Heap (MemoryStorage-based)
- All existing tests pass without modification
- Existing `tests/clustered_index_test.rs` stays (remains as standalone doc-test)

## Acceptance Criteria

### Functional

- [ ] `CREATE TABLE t (id INT PRIMARY KEY, ...) ENGINE=InnoDB CLUSTERED` parses & dispatches to ClusteredTable
- [ ] INSERT into CLUSTERED table stores row in B+ Tree leaf ordered by PK
- [ ] `SELECT WHERE pk = ?` from CLUSTERED table executes O(log N) lookup
- [ ] `SELECT WHERE pk BETWEEN ? AND ?` returns ordered range scan
- [ ] UPDATE on CLUSTERED table preserves B+ Tree ordering
- [ ] DELETE on CLUSTERED table frees B+ Tree entry
- [ ] Heap + Clustered tables coexist in single database

### Performance

- [ ] 1M-row INSERT benchmark: ClusteredTable ≥ Heap (INSERT is sequential append either way)
- [ ] PK lookup benchmark: ClusteredTable 10× faster than Heap for 1M+ rows
- [ ] Range scan benchmark: ClusteredTable produces sorted output in O(log N + k)
- [ ] `docs/releases/v3.11.0/perf/CLUSTERED_INDEX_PERF.md` documents ≥3 benchmarks with reproducible results

### Test Coverage

- [ ] `cargo test --package sqlrustgo --test clustered_main_path_test` — 8+ tests PASS
- [ ] `cargo test --package sqlrustgo --test cluster_index_main_path_test` — 5+ tests PASS
- [ ] No regression: existing 7/7 `tests/clustered_index_test.rs` still PASS

### Documentation

- [ ] `docs/releases/v3.11.0/perf/CLUSTERED_INDEX_PERF.md` published
- [ ] `docs/governance/debt/debt-registry.yaml` F-23 state: VERIFIED → CLOSED
- [ ] `ISOLATED_MODULES.md` F-23 row removed from isolated list

## Scope Boundaries

### IN SCOPE (V311-01)

- In-memory ClusteredTable with B+ Tree leaves
- CREATE TABLE option to opt in
- O(log N) PK lookup via B+ Tree
- O(log N + k) range scan via B+ Tree iteration
- Mixed Heap + Clustered tables in same instance
- Performance benchmarks

### OUT OF SCOPE (deferred to v3.12+)

- **Page-based disk persistence** (current stays in-memory)
- **WAL integration for ClusteredTable** (would need a new log record type)
- **MVCC compatibility** (would need visibility tracking in B+ Tree leaves)
- **Multi-column primary keys** (only single-column PK for V311-01)
- **Clustered secondary indexes** (existing hash/BTree secondary indexes remain as separate structure)
- **REPLACE INTO optimization** (out of scope)

These are spelled out as future work in the design document.

## Estimated Effort

| Step | Estimate | Complexity |
|------|----------|------------|
| 1. Parser ENGINE= syntax | 8h | Low |
| 2. ClusteredTable impl | 24h | Medium (StorageEngine trait surface) |
| 3. ClusteredBTree (row-storing leaves) | 16h | Medium (split/merge logic) |
| 4. ExecutionEngine routing | 8h | Low |
| 5. Tests | 16h | Medium |
| 6. Benchmarks + docs | 8h | Low |
| **Total** | **80h** | (matches V311-01 plan) |

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| StorageEngine trait is large (40+ methods) | High | Phase 1: implement subset used by CREATE/INSERT/SELECT/DELETE/UPDATE; defer rarely-used methods to follow-up PRs |
| Mixed-backend transactions | Medium | V311-01: cluster tables don't participate in transactions (use `MemoryStorage::begin_transaction` semantics); document clearly |
| Concurrent INSERT to ClusteredTable | Medium | RwLock-based serialization (same as MemoryStorage); benchmark shows acceptable throughput |
| Page-based disk backed storage regression | Low | V311-01 v1 is in-memory; disk-persistence is v3.12+ scope |
| v3.11.0 RELEASE timeline risk | High | Implement minimum viable subset (steps 1-4 above) for V311-01 v1; defer benchmarks to v311-01-perf follow-up |
