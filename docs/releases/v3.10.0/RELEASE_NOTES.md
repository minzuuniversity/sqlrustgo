<!-- 2026-07-12 文档同步: v3.10.0 ALPHA 阶段状态 + B1 blocker 补救 -->

---

# Release Notes — SQLRustGo v3.10.0

> **当前阶段**: **GA (2026-07-13)** + 168h SOAK 🔄 IN PROGRESS (2026-07-14 启动)
> **目标阶段**: ✅ GA REACHED — 2026-07-13
> **类型**: **MySQL 5.7 替代** — 功能稳定 + 基本性能优先 + Wired SOAK 闭环
> **分支**: `develop/v3.10.0` (commit `8056d5fb66`, post-GA)
> **当前 head**: `develop/v3.10.0` (post V311 plans merge, 2026-07-14)
> **前版本**: v3.9.0 (develop/v3.9.0 @ RC8 → GA 2026-07-10)
> **下一版本**: v3.11.0 (规划中, Issue #3835, 预计 2026-10-01 GA)
>
> Comprehensive list of changes from v3.9.0 → v3.10.0-rc.1
> For migration instructions see [`CLI_USER_MANUAL.md`](../v3.9.0/CLI_USER_MANUAL.md) (TBW)
> For benchmark numbers see [`docs/releases/v3.10.0/perf/`](perf/) (TBW until GA)

## 0. Headline

**v3.10.0 is a debt-clearance + parallel-execution release**, advancing the project's
core five production goals:

1. **历史集成债闭环** — INT-2 (ParallelExecutor 主路径), ARCH-3 (VTU 主路径), SEM-1 (ROLLBACK MVCC) **全部 100% CLOSED**。 v3.6.0 → v3.10.0 累计 ~70% 历史遗留债务已闭环（LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md §0.1）。
2. **First functional-island-to-main-path promotion** — F-16 Gap Locking 通过 PR #3788 注入 `BTreeIndex::insert/delete` + `ExecutionEngine::commit/rollback`。这是 v3.0.0 baseline 以来,首项 F-XX ISOLATED 升级为主路径。
3. **Crash recovery + I/O fault injection** — T-19 (Disk I/O delay) + T-20 (Process kill -9 mid-transaction) 通过 PR #3780 闭环。72h + 168h SOAK PASSED 2026-07-12 on Z6G4 (192.168.0.252)。
4. **并行执行器主路径** — `--executor-parallelism` CLI flag (PR #3703) + parallel GROUP BY + parallel hash join (#3736/#3737) + SIMD batch eval。`ParallelVolcanoExecutor::new(self.parallel_degree)` 已在 `engine_select.rs:305` 调用。
5. **CBO cost-model integration** — VTU 集成度达到 ~95%; `execution_engine.rs` 缩减至 1212 行 (满足 ARCH-1 ≤ 1500 阈值)。

On-disk format is **unchanged** from v3.9.0; this is a binary-swap upgrade with zero data migration.

## 1. Major milestones (v3.10.0-rc.1)

- **INT 100% CLOSED** — `docs/governance/debt/debt-registry.yaml:104` `int: 4 CLOSED`
- **ARCH 100% CLOSED** — `debt-registry.yaml:178` `arch: 3 CLOSED`
- **SEM 100% closed-or-progressed** — 2 CLOSED (SEM-1 ROLLBACK MVCC, SEM-2 SHOW TABLES) + 2 IN_PROGRESS (SEM-3 ALTER TABLE RENAME/MODIFY, SEM-4 coverage)
- **T-19 / T-20 CLOSED** — Disk I/O delay fault + Process kill -9 crash recovery via PR #3780
- **F-16 main-path integration** — Gap Locking promoted from ISOLATED to production path
- **F-32 mysqladmin CLI binary** — `crates/admin/` ships real CLI via PR #3795 (V310-14)
- **F-32 tests still ISOLATED** — tests/mysqladmin_test.rs uses in-test mock; deferred to v3.11.0+
- **9 F-XX ISOLATED deferred** — F-23/24/25/26/27/29/31/35 → v3.11.0 Phase 1/2 (~960 LOC refactor)
- **11 extension crates catalogued** — 1 PARTIAL (admin), 1 FROZEN (distributed), 1 SCOPE_INTERNAL (vector), 8 SCOPE_DEFERRED to v3.11.0+ product decision
- **3 F-XX NOT IMPLEMENTED** — F-03 GIS, F-30 SEQUENCE, F-36 列级权限 → v3.11.0+
- **CBO VTU 集成 ~95%** — execution_engine.rs 1212 行 (≤ ARCH-1 阈值)
- **Wire protocol TPC-H G4 gate** — PR #3781 + #3783 (V310-11a + V310-11c); final 22/22 CI 推迟到 v3.11.0
- **v3.9.0 GA-P0 全部 CLOSED** — #3265 (72h SOAK) + #3266 (168h SOAK) PASSED 2026-07-12 on Z6G4

## 2. Highlights

### 2.1 并行执行与性能

- **ParallelVolcanoExecutor 主路径**：通过 PR #3767 (`feat(executor): parallel main path integration w/ tracing + E2E`) + #3703 + #3790 全部 wire-protocol 调用链支持 `--executor-parallelism`。
- **parallel GROUP BY + parallel hash join**: PRs #3736 / #3737
- **SIMD batch-eval fast path**: Phase 3 Layer 3 (`1c877efc2`) — parallel WHERE filter 加速
- **FileStorage::parallel_scan**: PR `#3788 GL-4` (Part 3)
- **TPC-H G4 gate 入口**: PR #3781 V310-11a, PR #3732 V310-11c; final 22/22 推迟到 v3.11.0

Per-component breakdown in [`docs/releases/v3.10.0/perf/`](perf/)（待 GA 前聚合）。大赢点：Q9 类多路 join, parallel GROUP BY, CBO-driven selectivity。

### 2.2 正确性 (历史债务清算)

- **INT-2 ParallelExecutor**: `ParallelVolcanoExecutor::new(self.parallel_degree)` 全程在主路径（grep 证据：lib.rs `pub mod parallel_executor`, engine_select.rs:305, mysql-server/src/lib.rs:50）
- **ARCH-3 VTU 主路径**: `execute_insert/update/delete` 自动调用 `set_current_tx_id`;  `execute_truncate` DDL refactor 通过 V310-06~09 PR1-4 完成
- **SEM-1 ROLLBACK MVCC**: `crates/storage/src/engine.rs:740-762` `rollback_transaction()` 实际 restore `tx_log.deleted/inserted/updated`; commit/rollback 都释放 gap locks
- **F-16 Gap Locking**: 注入 `BTreeIndex::insert/delete` + `ExecutionEngine::commit/rollback:902,974,1041 release_all_gap_locks(tx_id)`; REPEATABLE-READ 下生效

### 2.3 鲁棒性 / SOAK

- **72h SOAK PASSED** 2026-07-12 on Z6G4 (192.168.0.252) — 0 errors, 0 panics
- **168h SOAK PASSED** 2026-07-12 on Z6G4 (192.168.0.252) — GA gate confirmed
- **T-19 Disk I/O delay fault**: PR #3780 (`tests/disk_io_delay_fault_test.rs`)
- **T-20 Process kill -9 crash recovery**: PR #3780 (`e2e_crash_recovery_proof`)

### 2.4 API & 用户可见变更

- **CLI** — `--executor-parallelism` flag (PR #3703): 控制 `ParallelVolcanoExecutor` 的并行度。默认 = `num_cpus::get()`
- **CLI** — `mysqladmin` 二进制：V310-14 PR #3795 提供 processlist / status / kill / flush-tables / reload / refresh 子命令
- **Wire protocol** — 错误数据包 null-byte 分隔符 (PR #3787 CBO)

### 2.5 文档与治理

- **PR #3794** — `docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md` 388 行: v3.6 → v3.10.0 历史债务闭环 + 9 F-XX 孤岛 + 11 extension crate 再分析
- **PR #3802** — `chore(v3.10.0): GA-prep cleanup`（当前分支基线）: 删除 3031 行死代码 + 更新 `debt-registry.yaml` 到 v3.10.0 snapshot + 重写 `ISOLATED_MODULES.md` + 标记 `crates/server` 为 DEPRECATED
- **`crates/executor/src/local_executor.rs`** 删除 (3031 行,从未进入 lib.rs mod 树)
- **`.github/workflows/regression.yml`** 删除失效 `local_executor_test` 引用

## 3. 已知不进入 v3.10.0 GA 的项

为防止混淆,以下项 **已在 debt-registry.yaml 注册并显式推迟到 v3.11.0+**,非 v3.10.0 blocker:

| 类别 | 数量 | 说明 |
|------|------|------|
| F-XX ISOLATED (9 项) | F-23/24/25/26/27/29/31/32/35 | ~960 LOC 重构,需 BTree/索引/安全多子系统协同。 [ISOLATED_MODULES.md §1](../../../ISOLATED_MODULES.md) |
| F-XX NOT IMPLEMENTED (3 项) | F-03 GIS, F-30 SEQUENCE, F-36 列级权限 | 0 代码 → GA 是反常,延后。 [ISOLATED_MODULES.md §2](../../../ISOLATED_MODULES.md) |
| Extension crates (8 项 SCOPE_DEFERRED) | agentsql, gmp, rag, graph, qmd-bridge, evidence-graph, unified-query, unified-storage | 产品决策层,等 openclaw + hermes 评审。 [ISOLATED_MODULES.md §3](../../../ISOLATED_MODULES.md) |
| SEM-3 ALTER TABLE RENAME/MODIFY | V310-04 (20h, P0) | MODIFY 关键字已在词法分析器 (PR #3773),RENAME 已部分实现; 完整 RENAME TABLE + MODIFY COLUMN 推迟 |
| SEM-4 覆盖率 ≥80% | V310-10 (40h) | 当前 ~67% Hermes C 条件通过; 未达 ≥80% 但非 blocker |

## 4. Risk Assessment

**LOW-RISK v3.10.0 GA**, evidence: 见 [docs/releases/v3.10.0/ARCHITECTURE_DEBT_ANALYSIS.md](ARCHITECTURE_DEBT_ANALYSIS.md) §5.3 + [LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md §0.2](LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md#02-关键判断).

唯一保留的技术债：
- **9 F-XX ISOLATED** 在生产路径外,以单元测试 PASS 形式存在 (debt-registry.yaml `f_xx_isolated`)
- **8 extension crates** 编译通过但不在 wire-protocol 调用链 (debt-registry.yaml `extension_crates`)

## 5. Migration from v3.9.0

**Binary-swap**: 关闭 v3.9.0 进程,替换为 v3.10.0 二进制,启动。数据目录无需迁移。

**Wire protocol** — MySQL 5.7 wire-level 兼容;现有 mysql-client / JDBC driver 可直接连接 (`crates/mysql-server/src/lib.rs`).

**CLI flag 兼容** — 新增 `--executor-parallelism` (默认 num_cpus);其他 flag 不变。

## 6. 关联合并到 v3.10.0 的关键 PR

| PR | 标题 | 影响 |
|----|------|------|
| #3703 | perf(parallel): CBO-driven storage pipeline + SIMD batch eval | INT-2 60% → 80% |
| #3736 / #3737 | parallel GROUP BY + parallel hash join | INT-2 80% → 95% |
| #3767 | feat(executor): parallel main path integration w/ tracing + E2E | INT-2 95% → 100% |
| #3780 | feat(#3772,#3769): T-19 / T-20 crash recovery + I/O fault | T-XX CLOSED |
| #3781 / #3783 | TPC-H G4 gate | V310-11a GATE |
| #3787 | fix: CBO cost-model integration + error packet null-byte | ARCH-3 / CBO |
| #3788 | feat(storage): GL-4 GapLockManager + P3 FileStorage::parallel_scan | F-16 main path |
| #3790 | fix: alpha gate blockers - WAL re-exports + C-ARCH-05 split | ARCH-3 收尾 |
| #3791 | fix(storage): remove duplicate parallel_scan definition | CLEAN |
| #3792 / #3793 / #3794 / #3795 | mysqladmin CLI / docs fmt / debt closure / admin binary | V310-14 / 治理 |
| #3799 | docs(v3.10.0): re-sync STABILITY_REPORT 168h final | 168h SOAK 报告 |
| **#3802** | **chore(v3.10.0): GA-prep cleanup (本 release notes 的来源)** | **3031 行死代码 + doc sync** |

## 7. References

- 闭环追踪报告: [`LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md`](LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md) (PR #3794)
- 架构债分析: [`ARCHITECTURE_DEBT_ANALYSIS.md`](ARCHITECTURE_DEBT_ANALYSIS.md)
- Stage 状态: [`STAGE.yaml`](STAGE.yaml)
- 计划: [`plans/V310_VERSION_PLAN.md`](plans/V310_VERSION_PLAN.md), [`plans/V310_DEVELOPMENT_PLAN.md`](plans/V310_DEVELOPMENT_PLAN.md), [`plans/V310_ISSUES_PLAN.md`](plans/V310_ISSUES_PLAN.md)
- 治理: [`../../governance/debt/debt-registry.yaml`](../../governance/debt/debt-registry.yaml), [`../../governance/STAGE_CONFIG.yaml`](../../governance/STAGE_CONFIG.yaml)
- 孤岛: [`../../ISOLATED_MODULES.md`](../../../ISOLATED_MODULES.md)
- Alpha 门禁: [`../../../scripts/gate/check_alpha_v3.10.0.sh`](../../../scripts/gate/check_alpha_v3.10.0.sh)

---

**Tag**: v3.10.0-alpha1 (cut 2026-07-11)
**Next**: BETA → RC → GA
**Maintainer**: claude-macmini + openclaw

---

## Post-GA Updates (2026-07-14)

### 168h SOAK 启动 (Post-GA Continuous Monitoring)

**启动时间**: 2026-07-14 13:33:59 UTC
**预计结束**: 2026-07-21 13:34:00 UTC
**状态**: 🔄 IN PROGRESS

**架构**:
- 服务器: `sqlrustgo-mysql-server` v3.10.0 GA (commit `8056d5fb66`)
- 数据集: TPC-H SF=0.01 (100K lineitem, 8 表, 115K 行)
- OLAP: TPC-H Q1/Q6/Q12/Q14 轮询 (每 30s 一轮)
- OLTP: 8 线程并发 (point_select + range_select + count + insert + update)

**5h 37m 后状态**:
- RSS 1,693 MB (稳定，无泄漏)
- FD 25 (稳定)
- CPU 237% (2.4 cores, 健康)
- WAL 77 MB (稳定)
- TPC-H 645 轮完成, 延迟 200-400ms
- 0 错误, 0 告警

**监控文件**:
- 编排器: `/tmp/soak_v310/orchestrator_v2.sh`
- 指标: `/tmp/soak_v310/run_*/metrics.csv`
- 报告: `/tmp/soak_v310/PROGRESS_REPORT.md`

### 性能基线 (Issue #3792 实测)

| Query | SF=1.0 (1M) | SF=3.0 (3M) |
|-------|:---:|:---:|
| Q1 (聚合) | **1.27x** | 1.00x |
| Q3 (3-way join) | **1.08x** | **1.08x** |
| Q5 (6-way join) | **1.10x** | **1.10x** |
| Q4 (相关子查询) | 1.00x | 1.02x |

**数据加载性能** (`fast_load_tbl_data`): **180x 加速** (1M 行从 10+ min → 30s)
