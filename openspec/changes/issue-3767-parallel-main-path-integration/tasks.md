# Tasks — issue-3767-parallel-main-path-integration

> Following OpenSpec format: 1.1, 1.2, ... = Requirements; 2.1, 2.2, ... = Implementation.

## 1. VERIFICATION (audit existing state)

- [x] **1.1** Verify `crates/executor/src/lib.rs:12` has `pub mod parallel_executor` (C-ARCH-01: must be public)
- [x] **1.2** Verify `parallel_degree` field exists on `ExecutionEngine` (`src/execution_engine.rs:106`)
- [x] **1.3** Verify `set_parallel_degree()` public API exists (`src/execution_engine.rs:215-217`)
- [x] **1.4** Verify env var reading on `ExecutionEngine::new()` (`src/execution_engine.rs:154`)
- [x] **1.5** Verify CLI flag wiring (`crates/mysql-server/src/main.rs:99-102`)
- [x] **1.6** Verify `run_server_v2` propagates to env (`crates/mysql-server/src/lib.rs:3284-3287`)
- [x] **1.7** Verify `execute_select` guard in `engine_select.rs:262-280`

## 2. INSTRUMENTATION

- [ ] **2.1** Add `tracing::info_span!("parallel_filter_engaged", ...)` at the start of the parallel filter guard in `src/engine_select.rs:262-280`
- [ ] **2.2** Span fields: `degree = self.parallel_degree`, `rows_in = rows.len()`, `partitions = N`
- [ ] **2.3** Span closes when parallel filter completes

## 3. E2E TESTS

- [ ] **3.1** Create `tests/parallel_main_path_test.rs`
- [ ] **3.2** Test: `test_execute_select_parallel_degree_4_path_engaged`
  - Insert 600K rows
  - Run SELECT with WHERE clause
  - Capture tracing via `tracing::subscriber::with_default`
  - Assert span `parallel_filter_engaged` emitted
- [ ] **3.3** Test: `test_execute_select_parallel_degree_1_path_sequential`
  - Same setup with `parallel_degree=1`
  - Assert span NOT emitted
- [ ] **3.4** Test: `test_execute_select_results_match_parallel_1_vs_4`
  - Insert 600K rows with known pattern
  - Run SELECT with `parallel_degree=1` and `parallel_degree=4`
  - Assert result rows are identical (modulo float tolerance)
- [ ] **3.5** Test: `test_for_update_disables_parallel_path`
  - SELECT ... FOR UPDATE with `parallel_degree=4`
  - Assert parallel filter NOT engaged (sequential fallback)
  - Assert result matches sequential baseline
- [ ] **3.6** Test: `test_cli_flag_propagation`
  - Set `SQLRUSTGO_EXECUTOR_PARALLELISM=8`
  - Construct `ExecutionEngine::new()`
  - Assert `engine.parallel_degree() == 8`
- [ ] **3.7** Test: `test_set_parallel_degree_override`
  - Construct with env var = 4
  - Call `set_parallel_degree(2)`
  - Assert `engine.parallel_degree() == 2`

## 4. CI INTEGRATION

- [ ] **4.1** Add `tests/parallel_main_path_test.rs` to `cargo test` invocation
- [ ] **4.2** Verify test passes with `--features parallel-executor`
- [ ] **4.3** Verify test passes without feature (sequential fallback)

## 5. DOCUMENTATION

- [ ] **5.1** Add comment block at `engine_select.rs:262` explaining the layering
- [ ] **5.2** Update `EXECUTION_PIPELINE_REFACTORING.md` (if it exists) or create
      `docs/releases/v3.10.0/plans/PARALLEL_MAIN_PATH.md` describing the integration status
- [ ] **5.3** Add OpenSpec change summary to `CHANGELOG.md` mentioning the I-12 close

## 6. CLOSE

- [ ] **6.1** All tests pass under `cargo test --features parallel-executor`
- [ ] **6.2** PR created and merged into `develop/v3.10.0`
- [ ] **6.3** Comment on Issue #3767 with the PR link
- [ ] **6.4** Close Issue #3767 after merge

## Status

| Task | Owner | Status |
|------|-------|--------|
| 1.x (audit) | claude | ✅ Complete 2026-07-12 |
| 2.x (tracing) | claude | ⏸ Pending |
| 3.x (E2E tests) | claude | ⏸ Pending |
| 4.x (CI) | claude | ⏸ Pending (covered by 3.x) |
| 5.x (docs) | claude | ⏸ Pending |
| 6.x (close) | claude | ⏸ Pending |
