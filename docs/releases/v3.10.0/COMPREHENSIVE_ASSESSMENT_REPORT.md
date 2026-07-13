# v3.10.0 综合评估报告

**版本**: v3.10.0
**阶段**: RC (2026-07-13)
**类型**: **MySQL 5.7 替代** — 功能稳定 + 基本性能优先 + Wired SOAK 闭环
**维护者**: openclaw / claude-macmini
**更新日期**: 2026-07-13

---

## 0. 摘要

v3.10.0 是 SQLRustGo 项目从 v3.6.0 → v3.10.0 债务清理路线图的终点站。本版本聚焦于：

1. **历史集成债闭环** — INT-2 (ParallelExecutor), ARCH-3 (VTU 主路径), SEM-1 (ROLLBACK MVCC) 全部 CLOSED
2. **并行执行器主路径** — `ParallelVolcanoExecutor` 进入生产调用链
3. **崩溃恢复真实验证** — T-19 (I/O 故障注入) + T-20 (kill -9 恢复) 闭环
4. **MySQL 5.7 wire 协议兼容** — 标准 mysql-cli/mycli/connector-python 可直接连接
5. **72h + 168h SOAK** 通过 —— 生产级稳定性确认

---

## 1. 版本定位

### 1.1 战略定位: MySQL 5.7 替代

v3.10.0 的目标不是特性竞赛, 而是 **MySQL 5.7 在嵌入式/轻量级场景的可替代方案**。

| 维度 | v3.10.0 定位 | MySQL 5.7 对比 |
|------|-------------|----------------|
| 部署 | 单进程无守护, 嵌入式或 CLI 启动 | 独立 server + client |
| 协议 | MySQL wire protocol (兼容) | 原生 |
| SQL | SQL-92 子集 + MySQL 方言 | 完整 MySQL |
| 事务 | ACID + MVCC (内存引擎 + WAL) | ACID + Redo/Undo |
| 适用场景 | 嵌入式、开发/测试环境、CI/CD | 通用生产数据库 |

### 1.2 版本范围 (scope)

| 包含 | 不包含 (v3.11.0+) |
|------|-------------------|
| SQL-92 核心 (SELECT/INSERT/UPDATE/DELETE/CREATE/DROP/ALTER) | F-03 GIS 空间数据 |
| TPC-H SF=0.1 22/22 | F-30 SEQUENCE / nextval |
| 并行执行器 (GROUP BY + hash join + SIMD batch eval) | F-36 列级权限 |
| MySQL wire protocol 兼容 (COM_QUERY/COM_STMT_PREPARE) | Cypher 图形查询 |
| WAL + crash recovery (kill -9) | SIMD / Vector SQL |
| 72h + 168h SOAK verified | 高级 MySQL 函数 (窗口函数扩展) |
| Parallel scan (FileStorage) | 分布式事务 |
| Gap Locking (F-16 main path) | |
| mysqladmin CLI (processlist/status/kill/flush/refresh) | |
| sql_corpus 验证框架 (815/818 PASS, 99.6%) | |

---

## 2. 架构评估

### 2.1 模块架构 (当前代码)

```
sqlrustgo/                      # workspace root
├── crates/
│   ├── parser/                 # SQL parser (SQL-92 + MySQL dialect)
│   ├── lexer/                  # Tokenizer
│   ├── planner/                # Query planner (CBO integrated)
│   ├── optimizer/              # Cost-based optimizer
│   ├── executor/               # Volcano-style executor
│   │   ├── parallel_executor   # ParallelVolcanoExecutor ✅
│   │   ├── pipeline_executor   # Pipeline execution
│   │   ├── task_scheduler      # Task scheduling (rayon-based)
│   │   ├── thread_pool_registry # Thread pool management
│   │   ├── hash_join           # Parallel hash join
│   │   └── dml/                # INSERT/UPDATE/DELETE
│   ├── storage/                # Storage engine (Memory + File + WAL)
│   │   ├── bplus_tree/         # B+ Tree index
│   │   ├── wal/                # Write-Ahead Log (42 contract tests)
│   │   └── engine.rs           # ExecutionEngine (VTU integrated)
│   ├── transaction/            # Transaction management
│   ├── catalog/                # Metadata (ColumnDefinition, TableInfo)
│   ├── types/                  # Type system (Value, DataType)
│   ├── mysql-server/           # MySQL wire protocol server
│   ├── mysql-client/           # MySQL wire protocol client
│   ├── server/                 # DEPRECATED (replaced by mysql-server)
│   ├── sqlrustgo-cli/          # Unified CLI (serve/exec/repl/bench/diag)
│   ├── admin/                  # mysqladmin (processlist/status/kill/flush)
│   ├── sql-corpus/             # SQL corpus validation (815/818 ✅)
│   ├── security/               # Security (authentication)
│   ├── recovery/               # Crash recovery (T-19/T-20)
│   └── cache/                  # LRU cache
├── tests/                      # Integration + E2E + benchmark tests
│   ├── integration/            # Category-based integration tests
│   ├── e2e/                    # End-to-end scenarios
│   └── benchmark/              # Performance benchmarks
```

### 2.2 架构质量

| 指标 | 当前值 | 阈值 | 状态 |
|------|--------|------|------|
| execution_engine.rs 行数 | 1212 | ≤ 1500 (ARCH-1) | ✅ |
| Clippy 错误 | 0 | 0 | ✅ |
| Fmt 漂移 | 0 | 0 | ✅ |
| 架构不变式 (`check_arch_invariants.sh`) | PASS | PASS | ✅ |
| ARCH-3 无绕过 (`check_arch3_no_bypass.sh`) | PASS | PASS | ✅ |
| 架构语义债 (`check_arch_sem_debt.sh`) | PASS (drift allowed) | ≤ DRIFT | ✅ |
| 跨版本债 (`check_cross_version_debt.sh`) | PASS | PASS | ✅ |

---

## 3. 功能完整性评估

### 3.1 SQL 支持

| SQL 特性 | 状态 | 备注 |
|----------|------|------|
| SELECT (单表/多表/子查询) | ✅ 完整 | 含 JOIN/LATERAL/WITH |
| INSERT (VALUES/SELECT/SET) | ✅ 完整 | INSERT SELECT 已验证 |
| UPDATE (单表/多表/子查询) | ✅ 完整 | SET 子句支持子查询 |
| DELETE (单表/多表/子查询) | ✅ 完整 | WHERE IN 子查询支持 |
| CREATE TABLE | ✅ 完整 | 含 FK/UNIQUE/CHECK/INDEX |
| ALTER TABLE (RENAME/MODIFY/ADD/DROP) | ✅ 部分 | RENAME TABLE ✅, MODIFY COLUMN 词法就绪 |
| DROP TABLE | ✅ 完整 | |
| MERGE | ✅ 完整 | ARCH-2 修复后 |
| UNION / INTERSECT / EXCEPT | ✅ 完整 | C-2 三子任务 |
| ROLLBACK MVCC | ✅ 完整 | SEM-1 真正撤销 DML |
| Gap Locking | ✅ 主路径 | F-16 main path integration |
| TPC-H SF=0.1 22/22 | ✅ | v3.9.0 继承 |
| sql_corpus (815/818) | ✅ 99.6% | 3 个已知语法差异 |

### 3.2 并行执行

| 特性 | 状态 | 备注 |
|------|------|------|
| ParallelVolcanoExecutor | ✅ 主路径 | `--executor-parallelism` CLI flag |
| Parallel GROUP BY | ✅ | PR #3736 |
| Parallel hash join | ✅ | PR #3737 |
| SIMD batch eval | ✅ | Phase 3 Layer 3 |
| FileStorage parallel_scan | ✅ | PR #3788 |

### 3.3 稳定性 & 恢复

| 特性 | 状态 | 备注 |
|------|------|------|
| 72h SOAK | ✅ PASSED | Z6G4, 0 errors |
| 168h SOAK | ✅ PASSED | Z6G4, 0 errors |
| T-19 I/O 故障注入 | ✅ | PR #3780 |
| T-20 kill -9 恢复 | ✅ | 8 种场景全部 PASS |
| WAL 42/42 合约测试 | ✅ | 全覆盖 |
| OOM 保护 (VectorBatch) | ✅ | 分配限制 |

---

## 4. 测试评估

### 4.1 测试统计

| 类别 | 数量 | 状态 |
|------|------|------|
| `cargo test --lib` (no features) | 600+ | ✅ PASS |
| `cargo test --lib` (parallel-executor) | 609+ | ✅ PASS |
| WAL 合约测试 | 42/42 | ✅ PASS |
| Integration gate | 4/4 | ✅ PASS |
| SGL semantic checks | 5/5 | ✅ PASS |
| sql_corpus | 815/818 (99.6%) | ✅ PASS |
| `#[ignore]` (RC 有效) | 8 ≤ 10 | ✅ PASS |
| Cross-version OPEN debt | 0 | ✅ PASS |
| `cargo test --workspace --no-run` | ❌ 1 FAIL | expr_single_engine_test (pre-existing) |

### 4.2 测试架构

```
tests/                          # root manifest
├── integration/                # Integration tests by domain
│   ├── sql/                    # SQL parse + execute tests
│   ├── dml/                    # DML (INSERT/UPDATE/DELETE) tests
│   ├── ddl/                    # DDL (CREATE/ALTER/DROP) tests
│   ├── transaction/            # ACID + MVCC tests
│   ├── wire/                   # MySQL wire protocol tests
│   └── data/                   # Test fixtures (TPC-H tiny/SF)
├── e2e/                        # End-to-end scenarios
│   ├── e2e_beta_test.rs        # BETA E2E scenarios
│   ├── crash_recovery.rs       # kill -9 crash recovery
│   └── ...
├── benchmark/                  # Performance benchmarks
├── unit/                       # Unit tests
├── stress/                     # Stress/fuzz tests
│   ├── crash_monkey_test.rs
│   └── recovery_fuzzer_test.rs
└── disabled/                   # Intentional disabled tests (v3.11+)
```

---

## 5. 性能评估

### 5.1 性能基线 (待收集)

| 指标 | v3.9.0 | v3.10.0 | 退化 |
|------|--------|---------|------|
| TPC-H SF=0.1 总耗时 | ~2.3s | TBD | TBD |
| sysbench TPS (8t) | ~150 | TBD | TBD |
| sysbench TPS (16t) | ~290 | TBD | TBD |
| Gap Locking P99 | N/A | TBD | New |

### 5.2 性能风险

- 并行执行引入线程调度开销（短查询可能退化）
- CBO 新增路径尚未有完整 TPC-H SF=1 验证
- Gap Locking 为新增特性, P99 延迟基线需收集

---

## 6. MySQL 5.7 横向对比

| 维度 | SQLRustGo v3.10.0 | MySQL 5.7 | SQLite 3.x | PostgreSQL 16 |
|------|-------------------|-----------|------------|---------------|
| **部署模型** | 单进程, CLI 启动 | Server + Client | 嵌入式库 (lib) | Server + Client |
| **启动时间** | < 1s | ~5-10s | < 10ms | ~2-5s |
| **内存占用 (idle)** | ~15-30 MB | ~200-500 MB | ~2-5 MB | ~50-100 MB |
| **磁盘占用** | ~85 MB (binary) | ~1-2 GB | ~5 MB | ~500 MB |
| **SQL 标准** | SQL-92 子集 | SQL:2011 部分 | SQL-92 子集 | SQL:2023 广泛 |
| **事务** | ACID + MVCC (内存 + WAL) | ACID + MVCC (Redo/Undo) | ACID (锁) | ACID + MVCC (SSI) |
| **并行查询** | ✅ ParallelVolcanoExecutor | ✅ 多线程 | ❌ 单线程 | ✅ 并行 (多进程) |
| **MySQL 协议** | ✅ Wire-level 兼容 | ✅ 原生 | ❌ | ❌ |
| **TPC-H SF=1** | ~65% 查询实现 | 100% | 100% | 100% |
| **SOAK 验证** | 168h ✅ | 企业级 | N/A | 企业级 |
| **部署复杂度** | 低 (单二进制) | 高 | 低 | 中 |
| **适用场景** | 嵌入式、开发环境、CI | 通用生产 | 移动端、小型嵌入 | 通用生产(高级 SQL) |

### 6.1 优势

- **极低部署复杂度**: 单二进制, 零配置启动, `sqlrustgo-mysql-server serve`
- **MySQL 协议兼容**: 现有 mysql-cli / mycli / JDBC / connector-python 可直接连接, 无需修改客户端
- **嵌入式友好**: 内存占用约 15-30 MB, 适合容器化部署
- **快速启动**: < 1s 启动到接受连接, 适合 CI/CD 和开发环境
- **SOAK 验证**: 168h 生产级稳定性通过

### 6.2 差距

- **SQL 功能**: SQL-92 子集, 不支持窗口函数、CTE 递归、GIS、全文索引等 MySQL 5.7 特性
- **TPC-H SF=1**: 约 65% 查询实现（22 个中实现 14 个, 8 个待 V310-11）
- **并发能力**: 单进程架构, 不适用高并发 (>100 连接) 场景
- **持久化**: 内存引擎 + WAL, 不适用于 >10GB 数据场景
- **工具生态**: 无 mysqldump 等价工具, 无 replication, 无 partitioning

### 6.3 推荐的替代场景

| 合适 | 不合适 |
|------|--------|
| CI/CD 测试数据库 | 大规模生产数据库 (> 10GB) |
| 开发环境 MySQL 替代 | 高并发 OLTP (> 100 连接) |
| 嵌入式应用数据库 | 需要完整 MySQL 特性的场景 |
| 单用户/小团队工具 | 数据仓库/OLAP 场景 |
| 教学/演示环境 | 地理空间 (GIS) 应用 |
| Docker 容器内数据库 | 需要主从复制/高可用 |

---

## 7. 风险评估

### 7.1 剩余风险

| 风险 | 等级 | 说明 | 缓解 |
|------|------|------|------|
| 并行执行器新引入 bug | 低 | PEX 经过 72h+168h SOAK | 持续 SOAK + 回归测试 |
| WAL 恢复边界情况 | 低 | 42 合约全覆盖 | 已通过 T-20 8 场景验证 |
| Gap Locking 死锁 | 低 | parking_lot 实现 | F-16 测试覆盖 |
| 性能退化 | 中 | 需 TPC-H SF=1 完整验证 | 等待 V310-11 |
| 编译测试失败 | 高 | 1 个文件 pre-existing | RC blocker (修复中) |

### 7.2 安全评估

| 项目 | 状态 |
|------|------|
| SQL 注入防护 | ✅ (参数化查询 + 字符串转义) |
| 认证机制 | ✅ (MySQL native password) |
| TLS 支持 | ✅ (mysql-server TLS) |
| 审计日志 | TBD (v3.11+) |

---

## 8. 结论与建议

### 8.1 GA 可行性

v3.10.0 **具备 GA 基础**, 主要证据：
- 核心 SQL 功能完整 (TPC-H SF=0.1 22/22)
- 并行执行器已进入主路径
- 崩溃恢复真实验证 (kill -9, 8 场景)
- 168h SOAK 无错误
- MySQL wire 协议兼容
- 历史债务闭环 (INT/ARCH/SEM 100%)

### 8.2 GA 前必做

1. **测试编译错误修复** — `expr_single_engine_test.rs` 字段重复 (RC blocker)
2. **Coverage baseline** — `cargo llvm-cov --lib`
3. **Perf baseline** — vs v3.9.0 对比
4. **E2E shell 脚本** — 8/10 场景自动化
5. **GA_GATE_REPORT.md 最终化** — 填入实际指标

### 8.3 GA 后建议

- v3.11.0 聚焦 TPC-H SF=1 22/22 完整实现
- 收集生产环境反馈, 确定功能扩展优先级
- 评估分布式/多进程架构可行性 (v4.0 roadmap)

---

## 9. 参考资料

- `docs/releases/v3.10.0/STAGE.yaml`
- `docs/releases/v3.10.0/ARCHITECTURE.md`
- `docs/releases/v3.10.0/TEST_PLAN.md`
- `docs/releases/v3.10.0/GA_GATE_REPORT.md`
- `docs/releases/v3.10.0/EVIDENCE_STATUS.md`
- `docs/governance/STAGE_CONFIG.yaml`
- `docs/governance/debt/debt-registry.yaml`
- `ISOLATED_MODULES.md`
