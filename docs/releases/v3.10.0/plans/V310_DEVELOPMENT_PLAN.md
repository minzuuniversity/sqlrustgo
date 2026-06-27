# SQLRustGo v3.10.0 开发计划

> **版本定位**: MySQL 5.7 替代 — 功能稳定 + 基本性能优先
> **分支**: `develop/v3.9.0` @ `766f3202c`
> **创建日期**: 2026-06-25
> **目标**: v3.9.0 中 44 个 ignore 测试 + 跨版本历史遗留债务整合

---

## 0. 版本定位与核心原则

### 0.1 MySQL 5.7 替代版本要求

v3.10.0 作为 MySQL 5.7 的替代版本，核心要求：

| 维度 | 要求 |
|------|------|
| **功能完整性** | 常用 DML/DDL/DQL 完整，不含破坏性 bug |
| **事务正确性** | ACID 四项完整，MVCC/ROLLBACK 正确 |
| **基本性能** | TPC-H SF=0.01 完整正确；QPS 不显著退化 |
| **稳定性** | 24h+ soak 无错误；Crash recovery 正确 |
| **兼容性** | 常用 SQL 语法、MySQL wire protocol、错误格式兼容 |

### 0.2 v3.10.0 不做

- 新语法（Cypher CREATE/MERGE 等图查询扩展）
- SIMD / Vector SQL / 新优化器
- 高级 MySQL 函数（GIS、FEOLE、窗口函数扩展）
- 新索引类型（自适应哈希、聚簇索引的磁盘集成）

### 0.3 总工作量估算

| 类别 | 来源 | 任务数 | 工作量 |
|------|------|--------|--------|
| 核心 MySQL 兼容性 | 跨版本历史债务 | 11 项 | ~180h |
| ignore 测试修复 | v3.9.0 审计 | 13 项 | ~200h |
| 性能基线 | v3.9.0 未完成 | 1 项 | ~40h |
| 稳定性验证 | soak/crash | 1 项 | ~80h |
| **合计** | | **~26 项** | **~500h** |

---

## 1. MySQL 兼容性核心要求（v3.10.0 必做）

> **来源**: v3.6-v3.9 跨版本债务 + INT5_PLUS_DEBT_INVENTORY.md

### 1.1 C-1: DML 完整性（来自 F-1 ignore 审计）

**来源**: `tests/dml_integration_test.rs` (7 个 ignore)

| ID | 功能 | 测试 | 根因 | 修复要求 |
|----|------|------|------|----------|
| C-1a | INSERT ... SELECT | `insert_select_copies_rows` (L101), `insert_select_with_type_coercion` (L120) | MemoryStorage INSERT SELECT 路径 0 rows | 从源表读取行，插入目标表 |
| C-1b | UPDATE ... SET col = (SELECT ...) | `update_with_subquery_in_set` (L220) | UpdateStatement SET 子句不支持子查询 | 扩展 SET 解析支持 `(SELECT ...)` |
| C-1c | Multi-table UPDATE | `update_multiple_tables` (L236) | parser/executor 只处理单表 | 支持 `UPDATE t1, t2 SET ... WHERE ...` |
| C-1d | DELETE ... WHERE col IN (SELECT ...) | `delete_with_subquery_in_where` (L306) | DeleteStatement 无子查询支持 | 支持 IN (SELECT ...) 和相关子查询 |
| C-1e | Multi-table DELETE | `delete_multiple_tables` (L322) | parser/executor 单表限制 | 支持 `DELETE t1, t2 FROM t1 JOIN t2 ...` |

**验证**: `cargo test insert_select_copies_rows insert_select_with_type_coercion update_with_subquery_in_set update_multiple_tables delete_with_subquery_in_where delete_multiple_tables` → 全部 PASS

---

### 1.2 C-2: UNION 集合操作（来自 F-2 ignore 审计）

**来源**: `tests/union_set_operations_test.rs` (3 个 ignore)

| ID | 功能 | 测试 | 根因 | 修复要求 |
|----|------|------|------|----------|
| C-2a | INTERSECT | `intersect_returns_common_rows` (L257) | `Statement` 无 Intersect 变体 | 添加枚举变体 + parser + executor |
| C-2b | EXCEPT | `except_returns_left_minus_right` (L276) | `Statement` 无 Except 变体 | 同上 |
| C-2c | UNION ORDER BY/LIMIT | `order_by_after_top_level_union` (L299) | `UnionStatement` 缺 order_by/limit | 扩展结构体 + parser + executor |

**验证**: `cargo test intersect_returns_common_rows except_returns_left_minus_right order_by_after_top_level_union` → 全部 PASS

---

### 1.3 C-3: 事务 ACID 正确性（SEM-1 + F-4 核心）

> **最高优先级** — ACID 不完整不能作为生产替代

**来源**: `tests/dml_integration_test.rs` (2 个) + `tests/stored_proc_catalog_test.rs` (3 个) + ARCH_SEM_DEBT_REMEDIATION_PLAN.md SEM-1

| ID | 功能 | 测试 | 根因 | 修复要求 |
|----|------|------|------|----------|
| C-3a | ROLLBACK 真正撤销 DML | `transaction_rollback_undoes_dml` (L352), `transaction_update_then_rollback` (L372) | MemoryStorage ROLLBACK 只回滚 WAL，不撤销 DML 行 | MVCC snapshot restore 或等效机制 |
| C-3b | MemoryStorage 事务边界 | `test_trigger_executes_update` (L284), `test_trigger_executes_delete` (L315), `test_trigger_executes_insert` (L346) | MemoryStorage 无 begin/commit/rollback 实现 | 实现基本事务支持或修改 trigger 路径 |

**验证**: `cargo test transaction_rollback_undoes_dml transaction_update_then_rollback test_trigger_executes_update test_trigger_executes_delete test_trigger_executes_insert` → 全部 PASS

---

### 1.4 C-4: ALTER TABLE 完整性（SEM-3 历史债务）

> **来源**: ARCH_SEM_DEBT_REMEDIATION_PLAN.md §6

**Status**: OPEN since v3.0.0

| ID | 功能 | 现状 | 修复要求 |
|----|------|------|----------|
| C-4a | ALTER TABLE ADD/DROP COLUMN | ✅ 已实现 | — |
| C-4b | ALTER TABLE RENAME TABLE | ❌ stub | 实现跨 schema 重命名 |
| C-4c | ALTER TABLE RENAME COLUMN | ❌ stub | 实现列重命名 |
| C-4d | ALTER TABLE MODIFY COLUMN | ❌ stub | 实现列类型修改 |

**验证**: 4 类 ALTER TABLE 操作均有实际效果（非 stub）

---

### 1.5 C-5: 崩溃恢复验证（SEM-1 + T-20 历史债务）

> **来源**: ARCH_SEM_DEBT_REMEDIATION_PLAN.md SEM-1 + INT5_PLUS_DEBT_INVENTORY T-20

| ID | 功能 | 现状 | 修复要求 |
|----|------|------|----------|
| C-5a | Crash recovery matrix | 129 场景 in-memory mock | 真实 kill -9 进程级崩溃注入 |
| C-5b | 24h soak | 模拟延迟 | 真实查询 + 真实负载 |
| C-5c | Disk I/O delay fault | ❌ 未实现 (T-19) | 注入 I/O 延迟，验证超时行为 |

**验证**: `tests/crash_monkey_test.rs` + `tests/long_run_stability_72h_test.rs` 真实运行

---

## 2. 跨版本历史债务（v3.6-v3.9 遗留，非 P0 但需规划）

> **来源**: INT5_PLUS_DEBT_INVENTORY.md + ARCH_SEM_DEBT_REMEDIATION_PLAN.md

### 2.1 高优先级（影响生产正确性）

| ID | 功能 | 引入版本 | 现状 | 修复要求 |
|----|------|---------|------|----------|
| H-1 | ARCH-2 双路径（mysql-server vs bench-cli） | v2.6.0 | OPEN | 统一入口，两 binary 行为一致 |
| H-2 | ARCH-3 VTU 主路径剩余 5% | v3.5.0 | PARTIAL | `execute_truncate` 接入 VTU + 移除白名单 |
| H-3 | F-03 GIS 空间数据（Point/LineString/Polygon） | v2.0.0 | ❌ NOT IMPLEMENTED | 需全量实现（可选，v3.11） |
| H-4 | F-30 CREATE SEQUENCE / nextval | v2.0.0 | ❌ NOT IMPLEMENTED | 需全量实现（可选，v3.11） |
| H-5 | F-36 列级权限 | v2.0.0 | ❌ NOT IMPLEMENTED | 需全量实现（可选，v3.11） |

### 2.2 中优先级（影响 MySQL 兼容性）

| ID | 功能 | 引入版本 | 现状 | 修复要求 |
|----|------|---------|------|----------|
| M-1 | SEM-4 覆盖率测量标准化 | v3.0.0 | OPEN | 统一 `cargo llvm-cov` 方法，机器间 <5% 方差 |
| M-2 | I-11 CBO 代价模型完善 | v2.0.0 | PARTIAL | 3 rules → 完整 CBO |
|: M-3 | F-01 CREATE EVENT 事件调度器 | v2.0.0 | PARTIAL | 部分实现，cron 式调度未完成 |
|: M-4 | F-07 查询缓存 DML 失效 | v2.0.0 | PARTIAL | LRU OK，DML invalidation 测试缺失 |
|: M-5 | **INT-2 ParallelExecutor 生产路径** | v3.9.0 | **未解决** | `parallel_degree` 硬编码为 1，生产路径零调用者；需添加 `--parallel-degree` CLI + 修改 `ExecutionEngine::new()` / `execute_select()` |
|: M-6 | **INT-3 stored_proc expression_to_value 重复** | v3.9.0 | **未解决** | `stored_proc.rs` 169 行独立重实现，应 delegate 到 `executor::expr::eval_*` free functions；需重构 ~120 行 |

### 2.3 低优先级（可选功能）

| ID | 功能 | 引入版本 | 现状 |
|----|------|---------|------|
| L-1 | F-02 FULLTEXT 全文索引 | v2.0.0 | PARTIAL |
| L-2 | F-34 AES-256 存储加密 | v2.0.0 | PARTIAL |
| L-3 | F-18 INFORMATION_SCHEMA 完整 | v2.0.0 | PARTIAL |

---

## 3. ignore 测试完整清单（v3.9.0 审计）

> **来源**: `IGNORE_REGISTRY_2026-06-25.md`

### 3.1 功能类 ignore（应修复 → C-1 ~ C-4）

| 文件 | 行 | 测试 | 类别 | 状态 |
|------|----|------|------|------|
| `dml_integration_test.rs` | 101, 120 | INSERT SELECT | C-1a | 待修复 |
| `dml_integration_test.rs` | 220 | UPDATE subquery | C-1b | 待修复 |
| `dml_integration_test.rs` | 236 | Multi-table UPDATE | C-1c | 待修复 |
| `dml_integration_test.rs` | 306 | DELETE subquery | C-1d | 待修复 |
| `dml_integration_test.rs` | 322 | Multi-table DELETE | C-1e | 待修复 |
| `dml_integration_test.rs` | 352, 372 | ROLLBACK DML | C-3a | 待修复 |
| `union_set_operations_test.rs` | 257 | INTERSECT | C-2a | 待修复 |
| `union_set_operations_test.rs` | 276 | EXCEPT | C-2b | 待修复 |
| `union_set_operations_test.rs` | 299 | UNION ORDER BY | C-2c | 待修复 |
| `stored_proc_catalog_test.rs` | 284, 315, 346 | MemoryStorage tx | C-3b | 待修复 |
| `boundary_test.rs` | 32 | INT64_MIN 解析 | ✅ 已修复 | PASS |
| `boundary_test.rs` | 89 | Zero division | ✅ 已修复 | PASS |

### 3.2 Cypher 扩展类 ignore（v3.11+）

| 文件 | 行 | 测试 | 状态 |
|------|----|------|------|
| `graph_cypher_integration_test.rs` | 16 | label predicate bool | 待评估 |
| `graph_cypher_integration_test.rs` | 466 | CREATE keyword | v3.11+ |
| `graph_cypher_integration_test.rs` | 476 | MERGE keyword | v3.11+ |
| `graph_cypher_integration_test.rs` | 485 | 无向边 `-` | v3.11+ |
| `graph_cypher_integration_test.rs` | 494 | OPTIONAL MATCH | v3.11+ |

### 3.3 性能基准类 ignore（手动运行）

| 文件 | 数量 | 说明 |
|------|------|------|
| `qps_benchmark_test.rs` | 10 | QPS/TPS 基准，专用环境 |
| `bench_v380_point_agg.rs` | 6 | v3.8.0 性能基线 |
| `perf_eng_batched_insert_test.rs` | 3 | 批量插入性能 |

### 3.4 长时稳定性类 ignore（Z6G4 阻塞）

| 文件 | 数量 | 说明 |
|------|------|------|
| `tpch_soak_test.rs` | 4 | 5m-30m soak，72h/168h 需 Z6G4 |
| `long_run_stability_72h_test.rs` | 1 | 72h 稳定性，Z6G4 阻塞 |

### 3.5 Manual Oracle 类 ignore

| 文件 | 行 | 说明 |
|------|----|------|
| `oracle_g1_tpch_sha256.rs` | 138 | SHA256 oracle 生成，手动 |

---

## 4. 阶段计划

### Phase 0: 基础修复（2 周，~80h）

**目标**: 关闭所有 ACID 正确性 bug

| 任务 | 来源 | 工作量 | 验证 |
|------|------|--------|------|
| PredicateCompiler Column 修复 | ignore 审计 | ✅ 已完成 | 8/8 PASS |
| Boundary test INT64_MIN 修复 | ignore 审计 | ✅ 已完成 | 2/2 PASS |
| C-3: ROLLBACK 真正撤销 DML | SEM-1 | ~40h | 2 tests PASS |
| C-3b: MemoryStorage 事务边界 | F-4b | ~20h | 3 tests PASS |
| C-4: ALTER TABLE 完整性 | SEM-3 | ~20h | 4 类操作 PASS |

### Phase 1: DML 增强（2 周，~80h）

**目标**: 常用 DML 完整，支持子查询

| 任务 | 来源 | 工作量 | 验证 |
|------|------|--------|------|
| C-1a: INSERT SELECT | F-1a | ~20h | 2 tests PASS |
| C-1b: UPDATE subquery | F-1b | ~15h | 1 test PASS |
| C-1c: Multi-table UPDATE | F-1c | ~15h | 1 test PASS |
| C-1d: DELETE subquery | F-1d | ~15h | 1 test PASS |
| C-1e: Multi-table DELETE | F-1e | ~15h | 1 test PASS |

### Phase 2: UNION + 稳定性（2 周，~80h）

**目标**: SQL 集合操作 + 真实崩溃恢复

| 任务 | 来源 | 工作量 | 验证 |
|------|------|--------|------|
| C-2a: INTERSECT | F-2a | ~15h | 1 test PASS |
| C-2b: EXCEPT | F-2b | ~15h | 1 test PASS |
| C-2c: UNION ORDER BY/LIMIT | F-2c | ~15h | 1 test PASS |
| C-5a: 真实 Crash Matrix | T-20 | ~20h | 真实 kill -9 PASS |
| C-5b: 24h 真实 Soak | T-19 | ~15h | 真实负载 PASS |

### Phase 3: 性能基线 + GA 准备（2 周，~80h）

**目标**: 性能不退化 + 文档完整

| 任务 | 来源 | 工作量 | 验证 |
|------|------|--------|------|
| H-2: ARCH-3 VTU 剩余 5% | ARCH-3 | ~20h | 白名单移除 |
| M-2: CBO 代价模型完善 | I-11 | ~15h | cost-based 选择生效 |
| H-1: ARCH-2 双路径统一 | ARCH-2 | ~20h | 两 binary 行为一致 |
| TPC-H SF=0.01 22/22 | G1 | ✅ PASS | 22/22 PASS |
| 文档收口 | — | ~25h | GA 文档完整 |

---

## 5. v3.10.0 GA 门禁

| Gate | 主题 | 验证 |
|------|------|------|
| G1 | TPC-H 22/22 | `cargo test --test tpch_gate_test` → 22/22 |
| G2 | ACID 正确性 | `cargo test transaction_rollback_undoes_dml transaction_update_then_rollback test_trigger_executes_*` → PASS |
| G3 | DML 完整性 | C-1a ~ C-1e 全部 PASS |
| G4 | UNION 集合操作 | C-2a ~ C-2c 全部 PASS |
| G5 | ALTER TABLE 完整 | C-4a ~ C-4d 全部 PASS |
| G6 | Crash Recovery | C-5a 真实 kill -9 PASS |
| G7 | 24h Soak | C-5b 真实负载 0 errors |
| G8 | 72h Soak | Z6G4 或等效环境 |

---

## 6. 配套文档

| 文档 | 内容 |
|------|------|
| `IGNORE_REGISTRY_2026-06-25.md` | 完整 ignore 清单（已更新至 44 个） |
| `V390_DEVELOPMENT_PLAN.md` | v3.9.0 架构债/可靠性任务 |
| `ARCH_SEM_DEBT_REMEDIATION_PLAN.md` | ARCH-1~3 + SEM-1~4 债务详情 |
| `INT5_PLUS_DEBT_INVENTORY.md` | v3.0.0~v3.8.0 跨版本债务全量清单 |
| `LONG_STABILITY_TESTS_ANALYSIS.md` | 长时测试详细分析 |
| `V390_COMPREHENSIVE_ASSESSMENT.md` | v3.9.0 综合评估 |

### 5.1 C-6: 缺失的 `sqlrustgo` CLI 程序（严重文档漏洞）
见 `plans/V310_CLI_BINARY_PLAN.md`（Phase 1-4）。

### 5.2 C-7: ARCH-2 MergeExecutor 字符串重解析（正交，非阻塞）
见 `plans/V310_CLI_BINARY_PLAN.md`（Stage 2-3）。

## 6. 配套文档

| 文档 | 内容 |
|------|------|
| `plans/V310_CLI_BINARY_PLAN.md` | CLI binary 实现计划（C-6）+ ARCH-2（C-7） |
| `IGNORE_REGISTRY_2026-06-25.md` | 完整 ignore 清单（已更新至 44 个） |
| `V390_DEVELOPMENT_PLAN.md` | v3.9.0 架构债/可靠性任务 |
| `ARCH_SEM_DEBT_REMEDIATION_PLAN.md` | ARCH-1~3 + SEM-1~4 债务详情 |
| `INT5_PLUS_DEBT_INVENTORY.md` | v3.0.0~v3.8.0 跨版本债务全量清单 |
| `LONG_STABILITY_TESTS_ANALYSIS.md` | 长时测试详细分析 |
| `V390_COMPREHENSIVE_ASSESSMENT.md` | v3.9.0 综合评估 |
