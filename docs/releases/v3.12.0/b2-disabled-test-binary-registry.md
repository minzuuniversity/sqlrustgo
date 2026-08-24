# v3.12.0 B2_INTEGRATION_TESTS — Disabled Test Binary Registry

> **provenance**: generated_by=claude-code v3.12.0, generated_at=2026-08-22T00:50:00Z, source_run=v3.12.0-beta-warn-remediation, gate_baseline=scripts/gate/check_beta_v3.12.0.sh B2_INTEGRATION_TESTS
> **policy**: Anti-Fabrication-Policy-v1.0 + ADR-008 (Test Claim Transparency)
> **scope**: This registry tracks **integration test binaries** that fail in `cargo test --all-features --test '*'`. Distinct from `disabled-test-registry.md` (which tracks `#[ignore]` markers — a different mechanism controlled by `tests/baseline/ignore_registry.json`).
> **expiry_policy**: All entries expire **2026-09-30** (per Issue #4385 acceptance criteria, approved by user 2026-08-22). Reactivation path = unblock the linked root-cause issue + remove entry from this file + remove from `DISABLED_TESTS_LIST` in `scripts/gate/check_beta_v3.12.0.sh`.

This registry tracks integration test binaries that are **pre-existing failures** in `cargo test --all-features --test '*'` at the v3.12.0 BETA gate baseline. They are excluded from the B2_INTEGRATION_TESTS gate via the `--skip` mechanism in `scripts/gate/check_beta_v3.12.0.sh` (see the script's `DISABLED_TESTS_LIST` variable).

## Owner and reactivation

Each entry has:
- **owner**: the GitHub handle accountable for closing the root-cause issue before expiry
- **issue**: the upstream issue tracking the underlying bug
- **reactivation_path**: the precise change required to remove the entry

Removing an entry requires (a) closing the upstream issue, (b) running the test binary locally and confirming 0 FAILED, (c) deleting the row from this file, and (d) updating `DISABLED_TESTS_LIST` in `scripts/gate/check_beta_v3.12.0.sh`.

## Registry entries (33 test binaries, captured 2026-08-21 baseline, updated 2026-08-24)

| # | Test binary | Failing test(s) | Root cause | Owner | Issue | Reactivation path |
|---|---|---|---|---|---|---|
|  1 | `ddl_e2e_test` | `test_alter_table_alter_column_set_data_type_rejected`, `test_ddl_sequential_create_drop_create` | DDL ALTER / sequential CREATE-DROP-CREATE not fully implemented in v3.12 (see `tests/e2e/ddl_e2e_test.rs:359,373`) | openclaw | pre-existing (no issue) | Implement full DDL ALTER COLUMN + transaction-safe CREATE/DROP sequencing; verify both tests PASS |
|  2 | `diag_q11` | (1 test) | TPC-H SF=1 Q11 stock-level filter correctness (sqlrustgo=200000 vs SQLite=0) | openclaw | #4377 | Fix Q11 stock-level filter; verify Q11 row count MATCH against SQLite oracle |
|  3 | `diag_q11_3way` | (1 test) | Same Q11 root cause; 3-way join filter regression | openclaw | #4377 | Fix Q11 multi-way join predicate propagation; verify against SQLite oracle |
|  4 | `diag_q11_having` | (1 test) | Same Q11 root cause; HAVING clause residual | openclaw | #4377 | Fix Q11 HAVING evaluation after join; verify against SQLite oracle |
|  5 | `diag_q11_steps` | (1 test) | Same Q11 root cause; step-by-step planner diagnostic | openclaw | #4377 | Fix Q11 incremental evaluation; verify against SQLite oracle |
|  6 | `diag_q11_where` | (1 test) | Same Q11 root cause; WHERE-only diagnostic | openclaw | #4377 | Fix Q11 WHERE predicate; verify against SQLite oracle |
|  7 | `diag_q12` | (1 test) | TPC-H SF=1 Q12 shipping mode predicate residual (sqlrustgo=7 vs SQLite=2) | openclaw | #4378 | Fix Q12 shipping mode filter; verify row count MATCH against SQLite oracle |
|  8 | `diag_q12_deep` | (1 test) | Same Q12 root cause; deep nested CASE-WHEN regression | openclaw | #4378 | Fix Q12 deep CASE-WHEN evaluation; verify against SQLite oracle |
|  9 | `diag_q14_full` | (1 test) | TPC-H SF=1 Q14 promotion-effect date filter residual | openclaw | #4374 | Fix Q14 date predicate; verify against SQLite oracle |
| 10 | `diag_q14_only` | (1 test) | Same Q14 root cause; Q14-only diagnostic | openclaw | #4374 | Fix Q14 predicate; verify against SQLite oracle |
| 11 | `diag_q14_q16` | (1 test) | Cross-issue Q14 + Q16 (anti join) regression; both depend on canonical NOT IN path (PR #4364 fixed Q16 but Q14 still has residual) | openclaw | #4374 | Verify Q14 against SQLite oracle after Q16 fix lands |
| 12 | `diag_shipdate_type` | (1 test) | Q12 shipdate type cast issue (related to #4378) | openclaw | #4378 | Fix shipdate column type handling; verify against SQLite oracle |
| 13 | `e2e_canonical_subprocess` | `e2e_legacy_binary_sqlrustgo_deprecated` | Legacy `sqlrustgo` binary path test; superseded by `sqlrustgo-mysql-server`; not relevant to v3.12 Beta scope | openclaw | pre-existing (no issue) | Delete the test file or migrate to `sqlrustgo-mysql-server` harness |
| 14 | `eval_22_vs_sf01` | `eval_22_in_process_vs_sqlite` | TPC-H SF=1 Q22 global-sales-opportunity TIMEOUT (>1800s) — 7-way self-join doesn't terminate | openclaw | #4381 | Fix Q22 query plan / join order; verify completes in <1800s |
| 15 | `eval_22_vs_sqlite` | `eval_22_vs_sqlite_sf01_canonical` | Same Q22 root cause; cross-engine comparison | openclaw | #4381 | Same as #4381 fix |
| 16 | `int2_substance_parallel_test` | `test_int2_sequential_skips_parallel` (and 8 others, all insert PARALLEL_MIN_ROWS loops) | `tests/integration/sql/int2_substance_parallel_test.rs:68` runs `for i in 0..(PARALLEL_MIN_ROWS / 10)` = 200,000 INSERTs; with per-row SQL parsing/planning this is single-digit-minute per test. Not a deadlock — known slow test. Other 8 tests in the binary also do 1000-200000 INSERTs each. | openclaw | pre-existing (no issue) | Refactor: replace single-row INSERTs with `INSERT INTO t SELECT ...` bulk inserts, or use `engine.bulk_insert_records()` to bypass per-row parsing. Once total binary runtime < 30s, reactivate. |
| 17 | `parallel_main_path_test` | 7 tests, all call `populate()` which runs 600,000 single-row INSERTs | `tests/integration/sql/parallel_main_path_test.rs:43` — `fn populate(engine)` runs `for i in 0..ROWS` (ROWS=600,000) doing 600K `engine.execute("INSERT ...")` calls. Each test in the binary calls populate, so total binary is 4.2M+ INSERTs = estimated 30+ minutes per binary. | openclaw | pre-existing (no issue) | Use `engine.bulk_insert_records()` API or `INSERT INTO t SELECT ...` to skip per-row SQL parsing/planning. Once per-binary runtime < 30s, reactivate. |
| 18 | `io_delay_fault_test` | `test_no_delay_when_not_configured` | Slow query log fault injection test; pre-existing failure | openclaw | pre-existing (no issue) | Investigate slow query log fault injection path |
| 19 | `load_local_infile_test` | `test_load_local_infile_client_refuses` | LOAD DATA LOCAL INFILE wire test; pre-existing failure | openclaw | pre-existing (no issue) | Investigate client-side rejection path |
| 20 | `mysqladmin_e2e_test` | (1 test) | mysqladmin CLI e2e; pre-existing failure | openclaw | pre-existing (no issue) | Investigate mysqladmin e2e flow |
| 21 | `mysql_client_e2e_test` | (1 test) | mysql client e2e; pre-existing failure | openclaw | pre-existing (no issue) | Investigate client e2e flow |
| 22 | `oracle_g1_tpch_sha256` | `g1_tpch_sha256_baseline` | TPC-H SF=1 SHA-256 baseline oracle; needs `--generate-baseline` flag to produce baseline | openclaw | pre-existing (no issue) | Run with `cargo test --test oracle_g1_tpch_sha256 -- --ignored --generate-baseline` to produce baseline, then re-run normally |
| 23 | `oracle_g5_sem1` | 4 tests (`g5_sem1_savepoint_rollback_restores_state_oracle`, `g5_sem1_savepoint_nested_oracle`, `g5_sem1_update_rollback_restores_old_value_oracle`, `g5_sem1_delete_rollback_undo_log_oracle`) | SAVEPOINT / rollback undo-log oracle tests; pre-existing failures (SEM-1 follow-up) | openclaw | pre-existing (no issue) | Investigate savepoint/rollback undo-log path |
| 24 | `oracle_p34_parallel_executor` | 2 tests (`p34_all_categories_have_tests_oracle`, `p34_test_file_categories_oracle`) | Parallel executor test inventory oracle; pre-existing failures | openclaw | pre-existing (no issue) | Investigate parallel executor test inventory |
| 25 | `parallel_perf_baseline_test` | `test_above_500k_rows_parallel_engages`, `test_for_update_still_disables_parallel` (and others) | `tests/benchmark/parallel_perf_baseline_test.rs:67` — `populate()` runs `for i in 0..rows` (rows=500,000) single-row INSERTs. Per-binary ~30+ minutes. | openclaw | pre-existing (no issue) | Use `engine.bulk_insert_records()` API; once per-binary <30s, reactivate. |
| 26 | `l3_canonical_binary` | `l3_canonical_binary_serve_handshake_auth_and_query` | L3 canonical binary handshake + auth + query integration; pre-existing failure | openclaw | pre-existing (no issue) | Investigate handshake/auth/query integration path |
| 27 | `q13_subquery_repro` | `q13_bare_subquery_row_count`, `q13_in_subquery_filters`, `q13_not_in_subquery_excludes` | TPC-H Q13 subquery regression; pre-existing failures | openclaw | #4374 (parent TPC-H) | Fix Q13 subquery evaluation; verify against SQLite oracle |
| 28 | `q16_notin_subquery_regression` | `q16_canonical_notin_full`, `q16_canonical_subquery_only` | TPC-H Q16 NOT IN path regression; partially fixed by PR #4364 but residual | openclaw | #4378 (Q12/Q16 cross-issue), #4278 (Q16) | Verify against SQLite oracle; if PR #4364 insufficient, reopen Q16 fix |
| 29 | `q21_cell_regression_test` | `q21_cell_regression_sf01` | TPC-H Q21 cell-level regression; pre-existing failure | openclaw | #4374 (parent TPC-H) | Fix Q21 EXISTS-hash path; verify against SQLite oracle |
| 30 | `physical_backup_test` | `test_physical_backup_help`, `test_physical_backup_list_empty_directory`, `test_physical_backup_prune_dry_run`, `test_physical_backup_prune_help` | Physical backup CLI integration; pre-existing failures | openclaw | pre-existing (no issue) | Investigate physical backup CLI subcommand implementations |
| 31 | `q2_q17_repro_test` | `q17_avg_threshold_subquery` (hangs >1800s), `q2_correlated_min_over_join` (hangs) | TPC-H Q17 small-order-shortage TIMEOUT (#4379) + Q2 LIMIT clause (#4375). Both queries run >60s and never terminate. | openclaw | #4375 (Q2 LIMIT), #4379 (Q17 TIMEOUT) | Fix Q17 small-order-shortage query plan to complete in <30s; fix Q2 LIMIT clause to limit results |
| 32 | `bulk_insert_v2_routing` | (REACTIVATED 2026-08-24) — previously listed as compile fail due to `BinaryTableStorageV2` unresolved import. PR #4417 landed; test now compiles AND passes `cargo test --test bulk_insert_v2_routing` → 17 passed, 0 failed. **Reactivated** — removed from this registry and from `DISABLED_TESTS_LIST`. | openclaw | pre-existing (follow-up to #4417) | — |
| 33 | `bin_storage_compaction_roundtrip` | (REACTIVATED 2026-08-24) — same `BinaryTableStorageV2` compile fix as entry 32. Test compiles AND passes `cargo test --test bin_storage_compaction_roundtrip` → 1 passed, 0 failed. **Reactivated** — removed from this registry and from `DISABLED_TESTS_LIST`. | openclaw | pre-existing (follow-up to #4417) | — |

## Total: 33 entries (31 active + 2 reactivated-as-removed), all with expiry 2026-09-30

**17 entries linked to OPEN TPC-H BLOCKER issues** (#4374/#4377/#4378/#4381, parent #4374 covers Q14/Q6 too, plus q13/q16/q21 added in this iteration). These require the upstream TPC-H correctness work to close — out of scope for #4385 (Beta gate remediation).

**14 entries are non-TPC-H pre-existing failures** (ddl_e2e_test, e2e_canonical_subprocess, int2_substance_parallel_test, parallel_main_path_test, parallel_perf_baseline_test, oracle_g1_tpch_sha256, oracle_g5_sem1, oracle_p34_parallel_executor, io_delay_fault_test, load_local_infile_test, mysqladmin_e2e_test, mysql_client_e2e_test, l3_canonical_binary, physical_backup_test).

## V312-59-B-FOLLOWUP: Issue #4413 — per-binary timeout gate

V312-59-B-FOLLOWUP (this PR) implements the gate restructure from #4413:
1. **Per-binary timeout**: each enabled binary runs under `timeout 120` — a binary that hangs beyond 120s is counted as a FAIL (not an indefinite block)
2. **Gate upgraded `warn` → `check`**: B2_INTEGRATION_TESTS is now a blocking gate (`check`), not a `warn`
3. **Two entries reactivated**: `bulk_insert_v2_routing` and `bin_storage_compaction_roundtrip` compile and pass after PR #4417 fixed the `BinaryTableStorageV2` feature gate
4. **`load_local_infile_eagain_regression_test` reactivated** in `DISABLED_TESTS_LIST` — removed from the list since it now compiles; it still has 1 FAIL (state-leaking `Table 'region' already exists`) which is a pre-existing runtime issue, not a compile failure

Remaining registry shrink path (entries 16-18, 25, 27): `parallel_main_path_test`, `int2_substance_parallel_test`, `parallel_perf_baseline_test` require bulk-insert API refactor to reduce per-binary runtime below 30s.

## Provenance and audit trail

- **Source**: `cargo test --all-features --test '*' --quiet --no-fail-fast` at HEAD `3eecde94e8` (develop/v3.12.0)
- **Captured**: 2026-08-21T23:36:00Z
- **Failure extraction**: B2 baseline log `/tmp/b2_baseline.log` (823 lines, 17 unique test binaries with `error: test failed, to rerun pass`)
- **Evidence**: `scripts/gate/check_beta_v3.12.0.sh` B2_INTEGRATION_TESTS reads the test-binary column from this file via `awk` and emits `--skip <binary>` for each entry

## Gate integration

The `B2_INTEGRATION_TESTS` check in `scripts/gate/check_beta_v3.12.0.sh` uses a per-binary loop with `timeout 120`. Disabled binaries are enumerated in `DISABLED_TESTS_LIST`.

When an entry is reactivated (root cause fixed), remove the binary name from `DISABLED_TESTS_LIST` and mark the row as reactivated (do not delete the row for audit trail); the gate automatically re-enables that binary in the next run.

## Reactivation workflow (closed by RC)

1. **Resolve the linked issue** (e.g. close #4377 once Q11 row count matches SQLite oracle)
2. **Local verification**: `cargo test --all-features --test <binary>` → 0 FAILED
3. **Update this registry**: mark the row as REACTIVATED (do not delete)
4. **Update the gate**: remove the binary name from `DISABLED_TESTS_LIST` in `scripts/gate/check_beta_v3.12.0.sh`
5. **Run B2_INTEGRATION_TESTS** to confirm 0 failures
6. **Commit + PR** with reference to the closing issue

This workflow is a precondition for RC/GA promotion per STAGE.yaml `promotion_to_RC_requires` row `V312-59-B: B2_INTEGRATION_TESTS passes 0 failed (disabled-test registry empty or all expired)`.
