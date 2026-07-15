# V311-01 Task Checklist

## Phase 1: Parser + AST (8h estimate)

- [ ] 1.1 Add `pub enum StorageEngineSpec { Heap, Clustered }` to `crates/parser/src/parser.rs`
- [ ] 1.2 Add `storage_engine: Option<StorageEngineSpec>` field to `CreateTableStatement` at line 597
- [ ] 1.3 Update CreateTable parsing to recognize `ENGINE=InnoDB CLUSTERED` (or `ENGINE=InnoDB` = Heap default)
- [ ] 1.4 Add test: `parser_test_clustered_create_table` — parse + serialize round-trip

## Phase 2: ClusteredTable implementation (24h estimate)

- [ ] 2.1 Create `crates/storage/src/clustered_table.rs` with basic struct (pk_index, secondary_idx, table_info)
- [ ] 2.2 Implement `ClusteredTable::new()`, `new_with_pk_column()`
- [ ] 2.3 Implement `insert`, `scan`, `lookup_pk`, `range_scan_pk`, `delete`
- [ ] 2.4 Implement `StorageEngine` trait impl: create_table, drop_table, get_table_info, has_table, list_tables
- [ ] 2.5 Implement `StorageEngine` trait impl: update, update_if (delegating to delete+insert)
- [ ] 2.6 Implement `StorageEngine` trait impl: create_index, drop_index (secondary index support)
- [ ] 2.7 Implement PK uniqueness constraint check
- [ ] 2.8 Implement `parallel_scan` for ClusteredTable
- [ ] 2.9 Add unit tests: `clustered_table_test.rs` with 8+ tests
  - test_insert_and_lookup_pk
  - test_range_scan_pk
  - test_update_preserves_order
  - test_delete
  - test_pk_uniqueness_violation
  - test_secondary_index
  - test_full_scan_sorted_by_pk
  - test_parallel_scan

## Phase 3: ExecutionEngine routing (8h estimate)

- [ ] 3.1 Create `src/clustered_dispatcher.rs` with storage routing wrapper
- [ ] 3.2 Modify `ExecutionEngine` to track `clustered_tables: Arc<RwLock<HashMap<String, Arc<RwLock<ClusteredTable>>>>>`
- [ ] 3.3 Modify `execute_create_table` to register ClusteredTable when spec=Clustered
- [ ] 3.4 Modify `scan` method to dispatch on clustered_tables presence
- [ ] 3.5 Modify `insert/delete/update` methods to dispatch correctly
- [ ] 3.6 Tests: end-to-end CREATE+INSERT+SELECT+DELETE on CLUSTERED tables
- [ ] 3.7 Tests: mixed Heap + Clustered tables in same database

## Phase 4: Tests + benchmarks (16h estimate)

### 4.1 Functional tests (`tests/integration/storage/clustered_main_path_test.rs`)

- [ ] test_create_clustered_table_via_engine_innodb_clause
- [ ] test_insert_and_select_pk_lookup
- [ ] test_select_pk_range_scan_returns_sorted
- [ ] test_update_clustered_table_preserves_btree_ordering
- [ ] test_delete_from_clustered_table
- [ ] test_mixed_heap_and_clustered_tables_coexist
- [ ] test_pk_uniqueness_constraint_enforced
- [ ] test_clustered_table_secondary_index

### 4.2 Performance benchmarks (`tests/integration/tpch/clustered_perf_benchmark.rs`)

- [ ] bench_clustered_insert_1m_rows
- [ ] bench_clustered_pk_lookup_1m_rows
- [ ] bench_clustered_pk_range_scan_100k_rows
- [ ] bench_clustered_vs_heap_full_scan
- [ ] Generate `docs/releases/v3.11.0/perf/CLUSTERED_INDEX_PERF.md` with numbers

### 4.3 Regression

- [ ] No regression: full `cargo test --all-features` passes
- [ ] Existing `tests/clustered_index_test.rs` 7/7 still PASS (standalone test)

## Phase 5: Documentation (8h estimate)

- [ ] 5.1 `docs/releases/v3.11.0/perf/CLUSTERED_INDEX_PERF.md` (perf comparison + benchmarks)
- [ ] 5.2 Update `docs/governance/debt/debt-registry.yaml` — F-23 VERIFIED → **CLOSED**
- [ ] 5.3 Update `ISOLATED_MODULES.md` — remove F-23 from §1 isolated list
- [ ] 5.4 Update `openspec/changes/V311_VERSION_PLAN.md` — V311-01 → ✅ DONE
- [ ] 5.5 Update `docs/releases/v3.11.0/FEATURE_CHECKLIST.md` — V311-01 → ✅ DONE

## Phase 6: PR + merge (1h)

- [ ] 6.1 `git checkout -b fix/v311-01-f-23-clustered-index`
- [ ] 6.2 `git push backup` 
- [ ] 6.3 Create PR #34XX on Gitea 250
- [ ] 6.4 Lower approval requirement via admin API
- [ ] 6.5 Merge PR
- [ ] 6.6 Force-push to gitcode + gitee
- [ ] 6.7 Restore approval requirement to 2

## Phase 7: Cleanup (final)

- [ ] 7.1 Final verification: `cargo test --all-features` PASS
- [ ] 7.2 Update Issue #3422 (or appropriate) with PR link
- [ ] 7.3 Verify all mirrors synced
