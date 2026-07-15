# Tasks — Issue #3399

## 1. Move + rename 4 existing scripts into tests/e2e/
- [ ] 1.1 `git mv scripts/gate/e2e/e2e_01_basic_crud.sh tests/e2e/startup_connect.sh`
- [ ] 1.2 `git mv scripts/gate/e2e/e2e_02_tx_commit_rollback.sh tests/e2e/rollback_mvcc.sh`
- [ ] 1.3 `git mv scripts/gate/e2e/e2e_03_wal_crash_recovery.sh tests/e2e/kill9_recovery.sh`
- [ ] 1.4 `git mv scripts/gate/e2e/e2e_04_parallel_executor.sh tests/e2e/tpch_sf01.sh`
- [ ] 1.5 Update header comments in renamed scripts to reflect new scenario name.

## 2. Author 4 new E2E scripts for missing scenarios
- [ ] 2.1 `tests/e2e/alter_rename.sh` — CREATE + ALTER TABLE RENAME + verify.
- [ ] 2.2 `tests/e2e/union_set_ops.sh` — UNION/INTERSECT/EXCEPT against fixture tables.
- [ ] 2.3 `tests/e2e/backup_restore.sh` — backup a DB, drop, restore, verify data.
- [ ] 2.4 `tests/e2e/sysbench_wired.sh` — sysbench OLTP via wire protocol.

## 3. Update orchestration
- [ ] 3.1 Update `scripts/gate/e2e/run_all_e2e.sh` to scan `tests/e2e/*.sh` (or remove `scripts/gate/e2e/` mirror).
- [ ] 3.2 Decide fate of the other 4 existing scripts (`e2e_05`–`e2e_08`, `cte_query` / `json_vector` / `migration` / `savepoint_rollback`) — keep, move, or remove.

## 4. Verify gate
- [ ] 4.1 `bash -n` each new script in `tests/e2e/`.
- [ ] 4.2 Verify `find tests/e2e -name "*<scenario>*"` matches all 8 RC scenario names.
- [ ] 4.3 (Optional) Run `scripts/gate/check_rc_gate_v3.10.0.sh` R4 check in isolation to confirm `E2E_FOUND=8`.

## 5. Commit and push
- [ ] 5.1 Commit on a new branch `fix/r4-e2e-scripts` (or `style/r4-e2e-scripts`).
- [ ] 5.2 Push to Gitea, create PR titled "R4: 8 E2E shell scripts in tests/e2e (#3399)".
- [ ] 5.3 Wait for review/merge.

## 6. Close issue
- [ ] 6.1 Comment on #3399 with PR link + verification result.
- [ ] 6.2 PATCH #3399 state=closed.

## 7. Archive
- [ ] 7.1 `openspec archive issue-3399-r4-e2e-scripts`.
