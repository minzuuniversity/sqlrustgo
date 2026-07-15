# V311-02 Adaptive Hash Index v2 Task Checklist

## Phase 1: Engine accessor (4h)

- [ ] 1.1 Add `ahi: Arc<AdaptiveHashIndex>` field to `ExecutionEngine` struct
- [ ] 1.2 Initialize AHI in `engine_builder.rs::new()` and 4 other constructors
- [ ] 1.3 Add `pub fn ahi(&self) -> &Arc<AdaptiveHashIndex>` accessor
- [ ] 1.4 Build cleanly, AHI::new() default

## Phase 2: Wire record_access (4h)

- [ ] 2.1 In `src/engine_select.rs::filter_partitions_parallel`, after filter
- [ ] 2.2 Determine primary key column index from table_info
- [ ] 2.3 Serialize PK value to bytes
- [ ] 2.4 Call `ahi.record_access(table, &pk_bytes, 0, 0)` per matching row

## Phase 3: Tests (4h)

- [ ] 3.1 Create `tests/integration/storage/adaptive_hash_main_path_test.rs`
- [ ] 3.2 test_pk_lookup_records_ahi_access
- [ ] 3.3 test_pk_lookup_promotes_after_threshold
- [ ] 3.4 test_ahi_hit_rate_strictly_increases
- [ ] 3.5 test_invalidate_table_clears_ahi
- [ ] 3.6 test_point_lookup_recurring_queries_benefit
- [ ] 3.7 Add `[[test]]` entry to `Cargo.toml`

## Phase 4: Regression + Bench (4h)

- [ ] 4.1 `cargo test --release --test adaptive_hash_index_test` (7/7)
- [ ] 4.2 `cargo test --release --test anti_join_main_path_test` (5/5)
- [ ] 4.3 `cargo test --release --test q21_exists_hash_path_test` (2/2)
- [ ] 4.4 `cargo test --release --test q4_hash_semi_join_test` (4/4)
- [ ] 4.5 `cargo test --release --test cluster_index_main_path_test` (7/7)
- [ ] 4.6 `cargo test --release --test clustered_table_v1_test` (3/3)
- [ ] 4.7 `cargo test --release --test decorrelation_test` (8/8)
- [ ] 4.8 `cargo test --release --test decorrelation_v2_test` (8/8)

## Phase 5: Docs (2h)

- [ ] 5.1 `docs/releases/v3.11.0/perf/ADAPTIVE_HASH_INDEX.md` (algorithm + bench)
- [ ] 5.2 Update `docs/governance/debt/debt-registry.yaml`: F-24 VERIFIED → CLOSED
- [ ] 5.3 Update `docs/releases/v3.11.0/FEATURE_CHECKLIST.md`: V311-02 → DONE v2
- [ ] 5.4 Update `openspec/changes/V311_VERSION_PLAN.md` if relevant

## Phase 6: PR + merge (30min)

- [ ] 6.1 Branch `fix/v311-02-f-24-adaptive-hash-v2`
- [ ] 6.2 Push to backup
- [ ] 6.3 Create PR on 250
- [ ] 6.4 Lower approval → 0
- [ ] 6.5 Merge
- [ ] 6.6 Force-push to gitcode + gitee
- [ ] 6.7 Restore approval → 2

## Phase 7: Issues (10min)

- [ ] 7.1 Search/create V311-02 issue tracker on 250
- [ ] 7.2 Post completion comment on completion
- [ ] 7.3 Close the issue
