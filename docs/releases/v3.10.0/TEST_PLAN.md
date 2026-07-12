# v3.10.0 Test Plan

> **Version**: v3.10.0
> **Status**: BETA stage (2026-07-13)
> **Owner**: @yinglichina8848 / Claude Code
> **Last update**: 2026-07-13 (Phase 0 implementation per DeepSeek feedback)
> **Related**: V310_DEVELOPMENT_PLAN.md, V310_ISSUES_PLAN.md, V310_10_COVERAGE_PLAN.md, V310_CLI_BINARY_PLAN.md

This document is the **SSOT** for v3.10.0's test plan. It replaces the
"per-version test plan" requirement in `STAGE_CONFIG.yaml` and serves as
the authoritative reference for BETA/RC/GA gate promotion. The framework-level
gates (G1-G16) are defined in `docs/governance/STAGE_CONFIG.yaml`; this
document enumerates the *application integration* scenarios and the
`#[ignore]` debt plan.

---

## 1. Test Matrix (5 维度)

| 维度 | 覆盖范围 | 测试方法 | 验收标准 | Owner |
|------|---------|---------|---------|-------|
| **功能 (Functional)** | SQL DML/DDL/DQL 完整 + 跨版本债修复 (V310-01~04) | `tests/integration/sql/`, `tests/integration/dml/`, `tests/integration/ddl/`, `tests/integration/transaction/` | 100% 语法支持, 0 panic, V310-01/02/03/04 全部 unignore | @dev-A |
| **性能 (Performance)** | TPC-H SF=1 22 查询, sysbench oltp_read_write, gap-locking latency | `tests/benchmark/tpch_sf1_bench.rs`, `tests/benchmark/qps_bench.rs` | 22/22 PASS, QPS ≥ v3.9.0 × 0.95, gap lock P99 < 50ms | @dev-B |
| **兼容性 (Compatibility)** | MySQL 5.7 协议 (COM_QUERY, COM_STMT_PREPARE, COM_STMT_EXECUTE, COM_STMT_CLOSE) + 错误格式 | `tests/integration/wire/` + `tests/integration/mysql_client_*` + `crates/mysql-server/tests/prepared_stmt_params_test` | 标准客户端 (mysql CLI, mycli, mysql-connector-python) 可连接 + 全部 22 TPC-H 通过 | @dev-C |
| **稳定性 (Stability)** | 24h / 168h SOAK with sysbench oltp_read_write 8-32 threads | `scripts/stability/run_24h_soak.sh`, `scripts/stability/run_168h_soak.sh` + `tests/integration/long_run_stability_72h_test` | 0 crashes, 0 data corruption, RSS 增长 < 5%, FD 数恒定 | @qa |
| **恢复 (Recovery)** | Crash recovery (kill -9 mid-tx), WAL replay, backup/restore roundtrip | `tests/e2e/crash_recovery.rs`, `tests/integration/recovery_scenarios_test.rs`, `sqlrustgo-admin` 端到端 | 所有已提交事务恢复, 未提交事务回滚, backup→restore 数据 identical | @dev-D |

---

## 2. End-to-End Scenarios (应用集成目标 — 10 个)

源自 `V310_TEST_BINARY_GATE_AUDIT_REPORT.md §4.4`, 每个场景为 v3.10.0 替代 MySQL 5.7 的 **必需** 应用集成测试。

| ID | 场景 | 入口 | 预期结果 | Gate 引用 |
|----|------|------|----------|-----------|
| **E2E-01** | 启动 + 连接 + `SELECT 1` | `sqlrustgo-mysql-server serve` + `mysql` CLI | 1 row, 1, time < 100ms | BETA |
| **E2E-02** | TPC-H SF=0.1 22 queries | `sqlrustgo-mysql-server` + 22 SF=0.1 queries | 22/22 PASS, 总耗时 < 30s, 行数匹配 SQLite oracle | BETA + GA |
| **E2E-03** | 24h 稳定性 with sysbench | `sqlrustgo-mysql-server` + `sysbench oltp_read_write` 8-32 threads | 0 errors, 0 reconnects, RSS 平台期 | GA |
| **E2E-04** | `kill -9` mid-transaction recovery | server + kill -9 PID | 重启后: 已提交事务 visible, 未提交事务 invisible, WAL 已 replay | BETA |
| **E2E-05** | Backup + Restore roundtrip | `sqlrustgo-admin backup` + `sqlrustgo-admin restore` | restore 后表数据 / schema / indexes identical to backup-time | RC |
| **E2E-06** | sysbench prepare → run → cleanup | `sqlrustgo-mysql-server` + `sysbench oltp_read_write prepare/run/cleanup` | prepare 创建 sbtest, run 产生 QPS, cleanup drop sbtest, 0 leak | RC |
| **E2E-07** | `ALTER TABLE RENAME` (data preserved) | server + `ALTER TABLE t1 RENAME TO t1_renamed` | rename 成功, 数据完整, 索引保留 | BETA |
| **E2E-08** | `ROLLBACK MVCC` snapshot | server + `BEGIN; INSERT; ROLLBACK;` | rollback 后 SELECT 看不到 INSERT, 0 数据 corrupt | BETA |
| **E2E-09** | `UNION / INTERSECT / EXCEPT` | server + 3 个集合操作 queries | 结果集与 PostgreSQL oracle 一致, 行数匹配 | BETA |
| **E2E-10** | 168h 长期稳定性 | server + 168h sysbench sustained load | 0 errors, 0 resource leak, 0 deadlock | GA |

**验收要求**: 全部 10 个场景在 BETA 阶段有**自动化测试脚本** (`tests/e2e/e2e_NN_*.rs`) 纳入 `check_beta_gate.sh` 统一触发。

---

## 3. Performance Baseline (v3.9.0 → v3.10.0)

| 指标 | v3.9.0 基线 | v3.10.0 目标 | 测量方法 |
|------|------------|--------------|----------|
| TPC-H SF=0.1 总耗时 | ~2.3s | ≤ 2.5s (≤ 8% 退化) | `tests/benchmark/tpch_sf0_1_bench.rs` |
| TPC-H SF=1 QPS | ~647 (avg over 22 queries) | ≥ 614 (≥ 95%) | `tests/benchmark/tpch_sf1_bench.rs` |
| sysbench oltp_read_write TPS (8t) | ~150 | ≥ 142 (≥ 95%) | `tests/benchmark/qps_bench.rs` |
| sysbench oltp_read_write TPS (16t) | ~290 | ≥ 275 (≥ 95%) | `tests/benchmark/qps_bench.rs` |
| Gap Locking P99 latency | N/A (new in v3.10) | < 50ms | `tests/benchmark/gap_lock_bench.rs` |
| Memory leak (24h) | 0 leaks | 0 leaks, RSS 增长 < 5% | `tests/integration/long_run_stability_72h_test.rs` |
| WAL bounded | 0-12.9 MB | ≤ 100 MB sustained | `scripts/gate/check_g13_stability.sh` |

**性能回归阈值**: 任一指标退化 > 5% 触发 RC 阶段的 `check_rc_ga_gate.sh` 失败。

---

## 4. `#[ignore]` Debt Closure Plan

**当前状态**: v3.10.0 共有 **49 个 `#[ignore]`** 分布在 20 个测试文件。V310-12 子任务目标 = 0。**注意**: v3.9.0 是 44, v3.10.0 增加了 5 个, 但目标应降到 0 (有缺陷 + benchmark + 跳过).

### 4.1 49 个 `#[ignore]` 全清单 (按文件分组)

| # | 测试文件 | `#[ignore]` 数量 | 类别 | 修复优先级 | 修复策略 |
|---|---------|----------------|------|-----------|---------|
| 1 | `tests/qps_benchmark_test.rs` | 10 | PERF_BENCHMARK | 低 | 保留 --ignored 模式, 标注 `--release` 要求 |
| 2 | `tests/bench_v380_point_agg.rs` | 6 | PERF_BENCHMARK | 低 | 同上 |
| 3 | `tests/integration/mysql_tpch_test.rs` | 4 | KNOWN_GAP (MySQL server 真实环境) | 中 | 拆分为 mock + 真环境两组, mock 跑 CI |
| 4 | `tests/graph_cypher_integration_test.rs` | 4 | UNSUPPORTED (Cypher 扩展) | 低 | 标记 v3.11+ 路线图, V310-12 接受保留 |
| 5 | `tests/vector/src/hnsw.rs` | 4 | ISOLATED (F-XX HNSW) | 低 | 等 F-XX 路线图 (v3.11+) |
| 6 | `tests/integration/issue_3257_wal_fallback_test.rs` | 0 (已有) | - | - | - |
| 7 | `tests/operators/exists_correlated.rs` | 2 | KNOWN_GAP | 中 | V310-02 (UNION) 子任务修复 |
| 8 | `tests/integration/vector_storage_integration_test.rs` | 2 | ISOLATED (F-16 gap locking) | 中 | V310-04 (ALTER) 后重新测试 |
| 9 | `tests/integration/tpch_sf1_test.rs` | 2 | HARDWARE_BLOCKED | 高 | V310-11 (SF=1 闭环), 等 75GB+ 磁盘平台 |
| 10 | `tests/sqlrustgo_cli_soak_e2e_test.rs` | 2 | ENV_DEPENDENT | 中 | 用 skip_if! 宏替代 ignore |
| 11 | `tests/perf_eng_batched_insert_test.rs` | 2 | PERF_BENCHMARK | 低 | 保留 |
| 12 | `tests/long_run_stability_72h_test.rs` | 1 | LONG_RUN (5s smoke) | 低 | 保留 smoke, full 在 `run_168h_soak.sh` |
| 13 | `tests/integration/tpch_comparison_test.rs` | 1 | HARDWARE_BLOCKED | 高 | V310-11 后修复 |
| 14 | `tests/crash_monkey_test.rs` | 1 | LONG_RUN (100k iter) | 中 | 拆分为 quick (10k) + full (100k) |
| 15 | `tests/oracle_g1_tpch_sha256.rs` | 1 | MANUAL_ORACLE | 低 | 保留 `--gen` 模式, 文档化 |
| 16 | `tests/recovery_fuzzer_test.rs` | 1 | LONG_RUN (50k iter) | 中 | 同上 (quick + full 拆分) |
| 17 | `tests/multi_statement_test.rs` | 1 | UNSUPPORTED (multi-stmt) | 中 | V310-12 子任务 |
| 18 | `crates/parser/src/parser.rs` | 1 | KNOWN_GAP (FK constraint) | 中 | V310-04 (ALTER) 后测试 |
| 19 | `crates/storage/src/mmap_vector_store.rs` | 1 | ISOLATED (vector storage) | 低 | F-16 路线图 |
| 20 | `crates/executor/tests/hash_join_left_null_test.rs` | 1 | KNOWN_GAP | 中 | V310-02 后修复 |
| 21 | `crates/vector/src/parallel_knn.rs` | 2 | PERF_BENCHMARK | 低 | 保留 |
| **合计** | | **49** | | | |

### 4.2 三阶段收敛计划

| 截止时间 | 目标 | 方法 | 状态 |
|---------|------|------|------|
| **v3.10.0 BETA (2026-07-20)** | ≤ 30 个 | (a) 修复 issue #3282 等已 closed (#5 个); (b) 拆分 long-run/benchmark tests (#6 个); (c) 用 `skip_if!` 宏替代 env-dependent (#2 个) | TODO |
| **v3.10.0 RC (2026-08-05)** | ≤ 10 个 | 剩余全部 fix, UNSUPPORTED 移到 `tests/disabled/` | TODO |
| **v3.10.0 GA (2026-08-25)** | **0 个 in `tests/`** | 不可修复的移至 `tests/disabled/` with `IGNORE_REASON.md` 解释 | TODO |

### 4.3 `tests/disabled/` 目录规则

- **位置**: `tests/disabled/<file>.rs.bak` 或直接移到 `tests/disabled/`
- **每个文件配 `IGNORE_REASON.md`**: 解释为什么不可修复, 何时可以重新启用
- **CI skip**: `check_test_inventory.sh` 自动跳过 `tests/disabled/`
- **不计入 `#[ignore]` 计数**

---

## 5. Gate Integration (与 STAGE_CONFIG.yaml 对应)

| 阶段 | 必跑 gates | 必跑 E2E | 必跑场景 | 必跑性能 |
|------|-----------|---------|---------|---------|
| **DRAFT** | `check_docs_links.sh`, `cargo build` | (无) | (无) | (无) |
| **ALPHA** | `check_alpha_v3.10.0.sh` (15 checks) | (无) | (无) | (无) |
| **BETA** | `check_beta_gate.sh` (新), `check_arch_invariants.sh`, `check_arch3_no_bypass.sh`, `check_arch_sem_debt.sh`, `check_cross_version_debt.sh`, `check_int_debt.sh`, `cargo build/test/fmt/clippy` | E2E-01, 02, 04, 07, 08, 09 | TPC-H SF=0.1 22/22 | TPC-H SF=0.1 耗时 ≤ 2.5s |
| **RC** | `check_rc_ga_gate.sh`, `check_anti_fabrication.sh`, `check_full_gate_verification.sh`, `check_drift_not_pass.sh`, `check_integration_gate.sh`, (BETA 全部) | + E2E-05, 06 | + 24h stability smoke | 性能回归 ≤ 5% |
| **GA** | (RC 全部) + `check_architecture_freeze.sh`, `check_gate_self_verification.sh`, `check_gate_test_integrity.sh` | + E2E-03, 10 (168h 长期) | + 168h SOAK PASS | (生产 baseline 锁定) |

---

## 6. 测试目录重组 (Phase 2 计划)

```
tests/                                (root manifest 当前)
├── unit/                             [TODO] 单元测试 (从 src/ 移入)
├── integration/                      [TODO] 按功能领域
│   ├── sql/                          解析/执行
│   ├── dml/                          INSERT/UPDATE/DELETE
│   ├── ddl/                          CREATE/ALTER/DROP
│   ├── transaction/                  ACID + MVCC
│   ├── wire/                         MySQL 协议
│   ├── tpch/                         TPC-H 22 查询
│   └── admin/                        backup/restore/verify
├── e2e/                              [TODO] 10 个端到端场景
│   ├── e2e_01_startup_connect.rs
│   ├── e2e_02_tpch_sf01.rs
│   ├── e2e_04_kill9_recovery.rs
│   ├── e2e_07_alter_rename.rs
│   ├── e2e_08_rollback_mvcc.rs
│   ├── e2e_09_union_set_ops.rs
│   └── ...
├── benchmark/                        [TODO] 性能基准
│   ├── tpch_sf1_bench.rs
│   ├── qps_bench.rs
│   └── gap_lock_bench.rs
└── disabled/                         [TODO] 不可修复的 ignore 测试
    └── IGNORE_REASON.md
```

**迁移策略**: 不一次性重命名, 改为:
1. Week 1: 在 `tests/integration/sql/`, `tests/integration/dml/` 等创建子目录, 用 `tests/integration/sql/X.rs` 创建软链到原 `tests/X.rs` (避免破坏旧 cargo 行为)
2. Week 2: 软链全部就位后, 一次性删除原 `tests/X.rs` 并在 `Cargo.toml` 改为新路径
3. Week 3: 测试全部通过, 删除 `tests/integration/*_test.rs` 软链

---

## 7. v3.10.0 验收门禁 (复述 V310_ISSUES_PLAN.md §1 G1-G10)

- **G1** TPC-H SF=0.1 22/22 ✅ (v3.9.0 已 PASS)
- **G2** ACID 正确性 → 5 tests PASS
- **G3** 覆盖率 ≥ 80% per crate (V310-10)
- **G4** TPC-H SF=1 22/22 (V310-11, 硬件阻塞)
- **G5** DML 完整性 → 6 tests PASS (V310-01)
- **G6** UNION 集合操作 → 3 tests PASS (V310-02)
- **G7** ALTER TABLE → 4 类操作 PASS (V310-04)
- **G8** 真实 Crash Matrix (kill -9) PASS (V310-05, T-20)
- **G9** 24h 真实 SOAK 0 errors (V310-05)
- **G10** Wired-SOAK sysbench prepare/run PASS (V310-09)

---

## 8. References

- `docs/governance/STAGE_CONFIG.yaml` — 5 阶段门禁 SSOT
- `docs/releases/v3.10.0/plans/V310_DEVELOPMENT_PLAN.md` — 26 任务 / 500h / 4 阶段
- `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` — 12 个 V310 子 ISSUE
- `docs/releases/v3.10.0/plans/V310_10_COVERAGE_PLAN.md` — 覆盖率 ≥ 80% 计划
- `docs/releases/v3.10.0/plans/V310_CLI_BINARY_PLAN.md` — 二进制收敛
- `docs/releases/v3.10.0/V310_TEST_BINARY_GATE_AUDIT_REPORT.md` — 本计划的源分析

---

*Last updated: 2026-07-13 by Claude Code (hermes-agent)*
*Phase 0 implementation per DeepSeek review feedback (local://attachment-1)*
