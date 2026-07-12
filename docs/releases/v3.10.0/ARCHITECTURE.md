# SQLRustGo v3.10.0 Architecture

> **版本**: v3.10.0
> **阶段**: DRAFT
> **创建日期**: 2026-07-01
> **父版本**: v3.9.0 (RC8)
> **状态**: 设计与规划阶段 (架构基本继承 v3.9.0)

---

## 1. 总体架构 (继承自 v3.9.0)

v3.10.0 架构与 v3.9.0 保持一致, 主要在功能完整性、性能和稳定性上增量改进.

### 1.1 模块结构

```
sqlrustgo (主入口)
├── sqlrustgo-cli (NEW, v3.10.0 user-facing CLI, PR #C-6)
│   ├── serve  → sqlrustgo-mysql-server serve
│   ├── exec   → sqlrustgo-mysql-server exec
│   ├── repl   → sqlrustgo-mysql-server repl
│   ├── bench  → sqlrustgo-mysql-server bench
│   ├── gmp    → sqlrustgo-mysql-server gmp
│   ├── diag   → sqlrustgo-mysql-server diag
│   ├── backup → sqlrustgo-mysql-server backup
│   ├── restore → sqlrustgo-mysql-server restore
│   └── cli    → Phase 3 skeleton (TCP connect + handshake)
├── sqlrustgo-mysql-server (full-featured server)
└── sqlrustgo-admin (offline admin: backup/restore/pitr + mysqladmin: status/reload/refresh/flush-tables/processlist/kill)

crates/
├── parser/         # SQL 解析器 (含 INTERSECT/EXCEPT/RENAME COLUMN 等 v3.10.0 新增)
├── planner/        # 查询计划 (含 CBO 改进 v3.10.0 M-2)
├── optimizer/      # 优化器 (含 M-2 CBO 完善)
├── executor/       # 执行器 (含 v3.10.0 DML 完整性 C-1)
│   ├── merge.rs    # MERGE executor (ARCH-2 C-7 修复后)
│   ├── dml/        # INSERT/UPDATE/DELETE (含 v3.10.0 C-1)
│   ├── set_ops/    # UNION/INTERSECT/EXCEPT (含 v3.10.0 C-2)
│   └── transaction.rs # ACID 完整 (含 v3.10.0 C-3)
├── storage/        # 存储层 (含 v3.10.0 MemoryStorage 事务边界 C-3b)
├── transaction/    # 事务管理 (含 v3.10.0 ROLLBACK 真正撤销 C-3a)
├── recovery/       # 崩溃恢复 (含 v3.10.0 真实 kill -9 验证 C-5a)
├── catalog/        # 元数据 (含 v3.10.0 ALTER TABLE RENAME 完整 C-4)
├── types/          # 类型系统
├── storage/        # 存储引擎
├── parser-tokenizer/ # 分词器
└── expr-engine/    # 表达式求值器
```

### 1.2 关键架构决策 (v3.10.0 保持)

- **D-1**: 单进程, 嵌入式执行 (v3.9.0 决定, v3.10.0 继续)
- **D-2**: MySQL wire protocol (v3.9.0 决定, v3.10.0 继续)
- **D-3**: 内存存储 + 磁盘 WAL (v3.9.0 决定, v3.10.0 继续)
- **D-4**: 阶段控制 (Stage Control Framework, PR #3668)
- **D-5**: G1-G16 门禁 (PR #3669)

### 1.3 v3.10.0 关键架构变化

| 变化 | 任务 | 来源 |
| --- | --- | --- |
| DML 完整性 | C-1a ~ C-1e (5 项) | F-1 ignore 审计 |
| UNION 集合操作 | C-2a ~ C-2c (3 项) | F-2 ignore 审计 |
| ACID 正确性 | C-3a, C-3b (2 项) | SEM-1 + F-4b |
| ALTER TABLE 完整 | C-4b ~ C-4d (3 项 stub 实现) | SEM-3 |
| 真实崩溃恢复 | C-5a (3 项) | SEM-1 + T-20 + T-19 |
| mysqladmin CLI 二进制 | V310-14 (6 subcommands) | F-32 (v3.8.0 遗留, Issue #3768) |
| ARCH-2 双路径统一 | H-1 | ARCH-2 历史债务 |
| ARCH-3 VTU 剩余 5% | H-2 | ARCH-3 历史债务 |
| CBO 完善 | M-2 | I-11 历史债务 |

---

## 2. v3.10.0 新增模块

### 2.1 C-1: DML 完整性模块

```
crates/executor/src/dml/
├── insert.rs       # 已有, 需扩展 INSERT SELECT
├── update.rs       # 需扩展 SET 子句子查询 + 多表 UPDATE
├── delete.rs       # 需扩展 WHERE IN 子查询 + 多表 DELETE
└── multi_table.rs  # NEW, 多表 DML 支持
```

### 2.2 C-2: UNION 集合操作模块

```
crates/executor/src/set_ops/
├── union.rs        # 已有, 需扩展 ORDER BY/LIMIT
├── intersect.rs    # NEW
├── except.rs       # NEW
└── set_ops_common.rs # NEW, 公共子句处理
```

### 2.3 C-3: ACID 事务模块

```
crates/storage/src/transaction/
├── snapshot.rs     # NEW, MVCC snapshot
├── rollback.rs     # 需扩展 (C-3a: 真正撤销 DML)
└── memory_storage.rs # 需实现事务边界 (C-3b)
```

### 2.4 C-4: ALTER TABLE 完整

```
crates/catalog/src/alter_table.rs
├── rename_table.rs  # NEW, C-4b
├── rename_column.rs # NEW, C-4c
└── modify_column.rs # NEW, C-4d
```

### 2.5 C-5: 真实崩溃恢复

```
crates/storage/src/
├── io_delay.rs      # C-5c I/O 延迟 / 损坏 / 丢弃 故障注入 ✅ (T-19, Issue #3772)
│   ├── IoDelayConfig   — delay_ms, corruption_rate, dropout_rate
│   ├── IoFaultInjector — apply_read / apply_write 闭包包装
│   └── LcgRng          — 自制 LCG (零外部依赖)
tests/
├── process_kill_crash_test.rs  # T-20 WalStorage crash recovery ✅ (Issue #3769)
│   ├── test_kill_mid_insert_update_uncommitted
│   ├── test_kill_mid_delete_uncommitted
│   ├── test_committed_survives_crash
│   ├── test_committed_delete_survives_crash
│   ├── test_empty_transaction_crash
│   ├── test_mixed_workload_recovery_report
│   ├── test_large_batch_crash
│   └── test_multiple_crash_recovery_cycles
crates/recovery/src/
├── crash_inject.rs  # NEW, C-5a 真实 kill -9 注入
└── long_soak.rs     # NEW, C-5b 24h 真实负载
```

---

## 3. v3.10.0 数据流 (主要场景)

### 3.1 INSERT SELECT (C-1a)

```
SQL: INSERT INTO target SELECT * FROM source
   ↓ parse
InsertStatement { select: Some(SelectStatement { ... }) }
   ↓ plan
Plan::InsertSelect { target, source }
   ↓ execute
1. 扫描 source 表所有行
2. 转换行格式 (类型匹配)
3. 插入到 target 表
4. 返回 affected_rows
```

### 3.2 INTERSECT / EXCEPT (C-2a, C-2b)

```
SQL: SELECT col FROM t1 INTERSECT SELECT col FROM t2
   ↓ parse
SelectStatement { set_op: Some(SetOp::Intersect(t2_query)) }
   ↓ plan
Plan::Intersect { left, right, schema }
   ↓ execute
1. 执行 left query
2. 执行 right query
3. 哈希去重 (left 存在 AND right 存在)
4. 返回结果集
```

### 3.3 ROLLBACK 真正撤销 (C-3a)

```
BEGIN
INSERT row1
UPDATE row2
DELETE row3
ROLLBACK
   ↓
1. MemoryStorage 检查 transaction 状态
2. 如果 active: 撤销所有 DML
3. WAL 标记 rollback 事件
4. transaction 结束
```

### 3.4 ARCH-2 双路径统一 (H-1)

```
当前: mysql-server (含 begin/commit/rollback) + bench-cli (单 SQL)
v3.10.0: 统一入口, 两者通过同一 ExecutionEngine 路径
   sqlrustgo-cli exec "SELECT 1"
     ↓ 调用 sqlrustgo-mysql-server exec
     ↓ 调用 LocalExecutor::execute_select()
     ↓ 走与 bench-cli 相同的代码路径
```

---

## 4. 性能目标 (C-5 + H-2 + M-2)

| 指标 | v3.9.0 baseline | v3.10.0 目标 |
| --- | --- | --- |
| TPC-H SF=0.01 (22 queries) | 22/22 PASS | 22/22 PASS, 不退化 |
| QPS (单连接简单 SELECT) | 1000+ | ≥1000 (不退化) |
| 24h 真实 soak | 模拟 | 真实 0 errors |
| 真实 crash recovery | 部分 mock | 100% kill -9 恢复 |
| execution_engine.rs 行数 | 1471 | ≤1500 (C-ARCH-05 锁回) |
| Clippy warnings | 0 | 0 |
| Fmt drift | 0 | 0 |
| Ignore tests (functional) | 13 | ≤5 |
| 覆盖率 | 40% | ≥60% |

---

## 5. 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
| --- | --- | --- | --- |
| C-1a INSERT SELECT 性能退化 | 中 | 中 | 实现需 O(N) 而非 O(N²), 走 storage 批量插入 |
| C-3a ROLLBACK 大事务性能 | 高 | 中 | 分段回滚 + lazy 撤销 |
| C-5b 24h real soak 暴露未知 bug | 中 | 高 | 分阶段: 1h → 4h → 24h |
| H-1 ARCH-2 行为差异 | 中 | 中 | 完整测试覆盖 mysql-server + bench-cli |
| M-2 CBO 错误优化 | 低 | 高 | cost-based 选择必须有 oracle 对比 |

---

## 6. 与 v3.11+ 的关系

v3.10.0 不做的内容 (推到 v3.11+):
- F-03 GIS 空间数据 (Point/LineString/Polygon)
- F-30 CREATE SEQUENCE / nextval
- F-36 列级权限
- Cypher 扩展 (CREATE/MERGE/OPTIONAL MATCH)
- SIMD / Vector SQL
- 高级 MySQL 函数 (FEOLE, GIS, 窗口函数扩展)

---

## 7. 文档关系

| 文档 | 关系 |
| --- | --- |
| `V310_VERSION_PLAN.md` | 战略定位 (本文件依赖) |
| `V310_DEVELOPMENT_PLAN.md` | 任务清单 (本文件依赖) |
| `V310_CLI_BINARY_PLAN.md` | C-6 + C-7 详细实现 |
| `STAGE.yaml` | 当前阶段状态 |
| `CHANGELOG.md` | 变更历史 |
| `../governance/STAGE_CONFIG.yaml` | 5 阶段 framework |
| `../governance/G1-G16 mapping` | 门禁映射 |

---

*本文档由 claude-macmini 在 v3.10.0 DRAFT 阶段创建, 继承 v3.9.0 架构并标注 v3.10.0 新增变化.*
