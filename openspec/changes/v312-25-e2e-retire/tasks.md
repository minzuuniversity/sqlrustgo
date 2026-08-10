# V312-25: E2E 脚本去重与死脚本清理 — tasks

> **Status**: 🔵 OPEN — created 2026-08-09
> **Owner**: minimax
> **Expiry**: 2026-08-25

- [ ] 1.1 `git rm tests/e2e/startup_connect.sh`（byte-identical stale mirror of e2e_01）
- [ ] 1.2 `git rm tests/e2e/tpch_sf01.sh`（mirror of e2e_04）
- [ ] 1.3 `git rm tests/e2e/kill9_recovery.sh`（mirror of e2e_03）
- [ ] 1.4 `git rm scripts/gate/e2e/e2e_01_basic_crud.sh`
- [ ] 1.5 `git rm scripts/gate/e2e/e2e_02_tx_commit_rollback.sh`
- [ ] 1.6 `git rm scripts/gate/e2e/e2e_03_wal_crash_recovery.sh`
- [ ] 1.7 `git rm scripts/gate/e2e/e2e_04_parallel_executor.sh`
- [ ] 1.8 `git rm scripts/gate/e2e/e2e_05_savepoint_rollback.sh`（dead-code）
- [ ] 1.9 `git rm scripts/gate/e2e/e2e_06_cte_query.sh`（dead-code）
- [ ] 1.10 `git rm scripts/gate/e2e/e2e_08_migration.sh`（dead-code, mislabeled）
- [ ] 1.11 在 `tests/baseline/ignore_registry.json` 新增 10 条 retired 条目（带 owner + expiry）
- [ ] 1.12 写 `docs/releases/v3.12.0/V312-25_e2e_retire_report.md`，含关闭边界 4 项的实测输出
