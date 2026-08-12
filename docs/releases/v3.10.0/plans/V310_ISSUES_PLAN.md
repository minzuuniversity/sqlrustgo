# v3.10.0 Issue 总控计划

> **创建日期**: 2026-07-11
> **创建人**: Claude Code
> **分支**: `develop/v3.10.0` (forked from `main` at `23353c0c54`)
> **父文档**: [`V310_DEVELOPMENT_PLAN.md`](V310_DEVELOPMENT_PLAN.md), [ADR-013](../../../governance/adr/ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md)
> **目标**: 替代 MySQL 5.7，可生产

---

## 1. 总控 ISSUE

### [V310-MASTER] v3.10.0 总控 — MySQL 5.7 替代 + Wired SOAK 闭环

**状态**: OPEN
**分支**: `develop/v3.10.0`
**Milestone**: v3.10.0
**优先级**: P0
**估时**: ~500h (~26 项核心任务)

**目标**:
- 关闭 v3.9.0 条件通过项（G3 覆盖率 ≥80%, G4 TPC-H SF=1 22/22）
- 完成 Wired-SOAK 闭环（sysbench prepare/run 真接入）
- 修复所有 ignore 测试（DML 完整性、UNION 集合操作、ACID）
- 真实崩溃恢复（kill -9）+ 24h 真实 SOAK
- 可作为 MySQL 5.7 替代用于生产

**子任务清单** (12 个子 ISSUE):

| # | Issue | 主题 | 估时 | 优先级 |
|---|--------|------|------|--------|
| 1 | [V310-01] | DML 完整性 | 80h | P0 |
| 2 | [V310-02] | UNION 集合操作 | 45h | P0 |
| 3 | [V310-03] | ACID 事务正确性 | 80h | P0 |
| 4 | [V310-04] | ALTER TABLE 完整性 | 20h | P0 |
| 5 | [V310-05] | 真实崩溃恢复 + 24h SOAK | 80h | P0 |
| 6 | [V310-06] | Wired-SOAK DDL 修复 (PR1) | 40h | P0 |
| 7 | [V310-07] | Catalog 4 层重构 (PR2) | 80h | P0 |
| 8 | [V310-08] | DDL 执行路径实现 (PR3) | 80h | P0 |
| 9 | [V310-09] | Wire 协议握手修复 (PR4) | 60h | P0 |
| 10 | [V310-10] | 覆盖率提升至 ≥80% | 40h | P1 |
| 11 | [V310-11] | TPC-H SF=1 22/22 闭环 | 80h | P1 |
| 13 | [V310-13] | Beta 测试体系建立 | 40h | P0 |

**门禁达标要求**:
- G1 TPC-H SF=0.1 22/22 ✅ (v3.9.0 已 PASS，需保持不退化)
- G2 ACID 正确性 → 5 tests PASS
- G3 覆盖率 ≥80% per crate
- G4 TPC-H SF=1 22/22
- G5 DML 完整性 → 6 tests PASS
- G6 UNION 集合操作 → 3 tests PASS
- G7 ALTER TABLE → 4 类操作 PASS
- G8 真实 Crash Matrix (kill -9) PASS
- G9 24h 真实 SOAK 0 errors
- G10 Wired-SOAK sysbench prepare/run 真接入 PASS

| **Beta** | 4 周 | V310-05, 07, 08, 10, 13 | Crash + 24h SOAK + Catalog + 覆盖率 + 测试体系 |

## 2. 子 ISSUE 详细描述

### [V310-01] DML 完整性

**来源**: V310_DEVELOPMENT_PLAN.md §1.1
**优先级**: P0
**估时**: 80h
**依赖**: 无

**子任务**:
- [V310-01a] INSERT ... SELECT (C-1a) — 20h, 2 tests
- [V310-01b] UPDATE ... SET col = (SELECT ...) (C-1b) — 15h, 1 test
- [V310-01c] Multi-table UPDATE (C-1c) — 15h, 1 test
- [V310-01d] DELETE ... WHERE col IN (SELECT ...) (C-1d) — 15h, 1 test
- [V310-01e] Multi-table DELETE (C-1e) — 15h, 1 test

**验证**:
```bash
cargo test insert_select_copies_rows \
  insert_select_with_type_coercion \
  update_with_subquery_in_set \
  update_multiple_tables \
  delete_with_subquery_in_where \
  delete_multiple_tables
```

**完成判据**: 6 tests PASS

---

### [V310-02] UNION 集合操作

**来源**: V310_DEVELOPMENT_PLAN.md §1.2
**优先级**: P0
**估时**: 45h
**依赖**: 无

**子任务**:
- [V310-02a] INTERSECT (C-2a) — 15h
- [V310-02b] EXCEPT (C-2b) — 15h
- [V310-02c] UNION ORDER BY/LIMIT (C-2c) — 15h

**验证**:
```bash
cargo test intersect_returns_common_rows \
  except_returns_left_minus_right \
  order_by_after_top_level_union
```

**完成判据**: 3 tests PASS

---

### [V310-03] ACID 事务正确性

**来源**: V310_DEVELOPMENT_PLAN.md §1.3 (SEM-1)
**优先级**: P0 (最高)
**估时**: 80h
**依赖**: 无

**子任务**:
- [V310-03a] ROLLBACK 真正撤销 DML (C-3a) — 40h, MVCC snapshot restore
- [V310-03b] MemoryStorage 事务边界 (C-3b) — 20h, begin/commit/rollback 实现
- [V310-03c] Trigger 在事务内执行 (T-1/F-4c) — 20h

**验证**:
```bash
cargo test transaction_rollback_undoes_dml \
  transaction_update_then_rollback \
  test_trigger_executes_update \
  test_trigger_executes_delete \
  test_trigger_executes_insert
```

**完成判据**: 5 tests PASS, ACID 四项完整

---

### [V310-04] ALTER TABLE 完整性
| Gitea Issue | 标题 | 主题标签 |
|-------------|------|---------|
| #3721 | [V310-MASTER] v3.10.0 总控 | P0, architecture |
| #3722 | [V310-01] DML 完整性 | P0, executor |
| #3723 | [V310-02] UNION 集合操作 | P0, sql-semantics |
| #3724 | [V310-03] ACID 事务正确性 | P0, semantics |
| #3725 | [V310-04] ALTER TABLE 完整性 | P0, mysql-compatibility |
| #3726 | [V310-05] 真实崩溃恢复 + 24h SOAK | P0, stability |
| #3727 | [V310-06] Wired-SOAK DDL 修复 (PR1) | P0, architecture |
| #3728 | [V310-07] Catalog 4 层重构 (PR2) | P0, architecture |
| #3729 | [V310-08] DDL 执行路径实现 (PR3) | P0, executor |
| #3730 | [V310-09] Wire 协议握手修复 (PR4) | P0, mysql-server |
| #3731 | [V310-10] 覆盖率提升至 ≥80% | P1, coverage |
| #3732 | [V310-11] TPC-H SF=1 22/22 闭环 | P1, ga-p0-tpch |
| #3733 | [V310-12] 其他 ignore 测试 + 跨版本债 | P2 |

> **状态**: ✅ 全部创建于 2026-07-11 (Gitea 252)。

### [V310-05] 真实崩溃恢复 + 24h SOAK

**来源**: V310_DEVELOPMENT_PLAN.md §1.5 (SEM-1 + T-20)
**优先级**: P0
**估时**: 80h
**依赖**: V310-03

**子任务**:
- [V310-05a] 真实 Crash Recovery Matrix (C-5a) — 40h, kill -9 进程级崩溃注入
- [V310-05b] 24h 真实 SOAK (C-5b) — 30h, 真实查询 + 真实负载
- [V310-05c] Disk I/O Delay Fault (T-19) — 10h, 注入 I/O 延迟

**验证**:
- `tests/crash_monkey_test.rs` 真实运行 PASS
- 24h 真实负载 0 errors
- I/O 延迟注入超时行为正确

**完成判据**: G8 + G9 PASS

---

### [V310-06] Wired-SOAK DDL 修复 (PR1)

**来源**: ADR-013 §Decision PR1
**优先级**: P0
**估时**: 40h (~1 周)
**依赖**: 无

**目标**: parser 加 `CREATE DATABASE` / `DROP DATABASE` / `USE` AST 节点 + 语法解析

**子任务**:
- [V310-06a] AST 节点扩展（DatabaseStmt / DropDatabaseStmt / UseStmt） — 16h
- [V310-06b] Lexer/parser 语法分析 — 16h
- [V310-06c] `check_g16_ddl_syntax.sh` 新增 + 50 解析测试 PASS — 8h

**验证**: `bash scripts/gate/check_g16_ddl_syntax.sh` PASS

**完成判据**: 50+ 解析测试 PASS，DDL AST 节点齐全

---

### [V310-07] Catalog 4 层重构 (PR2)

**来源**: ADR-013 §Decision PR2
**优先级**: P0
**估时**: 80h (~2 周)
**依赖**: V310-06

**目标**: catalog 引入 `Database` 抽象 + `Catalog -> Database -> Schema -> Table` 4 层重构

**子任务**:
- [V310-07a] Database 抽象层引入 — 32h
- [V310-07b] 4 层 Catalog 树重构 — 32h
- [V310-07c] `check_p14_upgrade_test.sh` + 全 storage 回归 — 16h

**验证**:
- `check_p14_upgrade_test.sh` PASS
- 全 storage 路径无回归

**完成判据**: 4 层模型稳定，194 fixture test 全 PASS

---

### [V310-08] DDL 执行路径实现 (PR3)

**来源**: ADR-013 §Decision PR3
**优先级**: P0
**估时**: 80h (~2 周)
**依赖**: V310-07

**目标**: executor `execute_create_database` / `execute_drop_database` / `execute_use` 实现 + storage 路径 `data/<db>/<table>.tbl` 改造

**子任务**:
- [V310-08a] executor DDL 执行器 — 32h
- [V310-08b] storage data 路径 `data/<db>/<table>.tbl` 改造 — 32h
- [V310-08c] `check_arch_invariants.sh` + 194 fixture test 回归 — 16h

**验证**:
- `check_arch_invariants.sh` PASS
- 194 fixture test 全 PASS
- 一次性重生成所有 fixture + sha256 验证（用 `tests/baseline/tpch_hashes_v380.json` 机制）

**完成判据**: DDL 全路径可用，fixture 100% 稳定

---

### [V310-09] Wire 协议握手修复 (PR4)

**来源**: ADR-013 §Decision PR4
**优先级**: P0
**估时**: 60h (~1-2 周)
**依赖**: V310-08

**目标**: mysql-server wire 修 `seq = incoming_seq + 1` (PHASE 2) + capability flags (PHASE 3) + auth packet

**子任务**:
- [V310-09a] PHASE 2: seq number 修复 — 16h
- [V310-09b] PHASE 3: capability flags 协商 — 24h
- [V310-09c] auth packet 实现 — 12h
- [V310-09d] sysbench 真接入 + `check_g12_sysbench.sh` unignore — 8h

**验证**:
- `check_g12_sysbench.sh` unignore
- `mysql-cli` 本地连接握手成功
- sysbench oltp_read_write prepare/run 真接入

**完成判据**: sysbench prepare/run 真跑通，JDBC/ODBC 客户端可连接

---

### [V310-10] 覆盖率提升至 ≥80%

**来源**: ISSUE #3302 (v3.10.0 parser/executor coverage tracking)
**优先级**: P1
**估时**: 40h
**依赖**: 无

**目标**: 各 crate 覆盖率从 ~67% 提升至 ≥80%

**子任务**:
- [V310-10a] sqlrustgo-executor 68% → ≥80% — 16h
- [V310-10b] sqlrustgo-parser 60% → ≥80% — 16h
- [V310-10c] sqlrustgo-storage 78% → ≥80% — 8h

**验证**:
```bash
cargo llvm-cov --workspace --summary-only
# 所有 main crate ≥80%
```

**完成判据**: G3 (覆盖率) ✅ (无 conditional), G3 = PASS (强)

---

### [V310-11] TPC-H SF=1 22/22 闭环

**来源**: TPC-H_PARTIAL_RESULT.md v3.10.0 Phase 1
**优先级**: P1
**估时**: 80h
**依赖**: 无

**目标**: TPC-H SF=1.0 (6M 行) 22/22 PASS

**子任务**:
- [V310-11a] 修复 4 个 parser 解析错误 — 32h
  - Q7/Q8/Q9/Q12 子查询 in FROM 子句语法
  - OR 优先级问题
- [V310-11b] 实现其余 12 个未实现查询 — 40h
- [V310-11c] SF=1.0 22/22 全测试 — 8h ✅ 部分完成 (gate 入口 + 测试基础设施，commit `512b383c`，2026-07-12；22/22 PASS 仍需 V310-11a/b)

**验证**:
```bash
bash scripts/gate/check_tpch_sf1.sh
# 22/22 PASS
```

**完成判据**: G4 (TPC-H SF=1) ✅ (无 conditional), G4 = PASS (强)

---

### [V310-12] 其他 ignore 测试 + 跨版本债

**来源**: V310_DEVELOPMENT_PLAN.md §2 (跨版本债)
**优先级**: P2
**估时**: 60h
**依赖**: 无

**目标**: 关闭 P2 级 ignore 测试 + 跨版本债

**子任务**:
- [V310-12a] M-5 INT-2 ParallelExecutor 生产路径 — 20h
- [V310-12b] M-6 INT-3 stored_proc 重构 — 20h
- [V310-12c] H-2 ARCH-3 VTU 剩余 5% — 20h

**验证**: 全部 ignore 测试 unignore + cargo test PASS

**完成判据**: P2 债务全部关闭
### [V310-13] Beta 测试体系建立

**ISSUE**: #3372

**目标**: 在 Beta 阶段建立全面测试体系，解决测试只验证「SQL 能执行」而不验证「结果是否正确」的根本问题。

**现状问题**:
+ `crates/sqlancer`: ~100行骨架，从未使用
+ `sql_corpus/`: 103个SQL文件(7071行)，从未作为测试运行
+ 缺乏跨数据库语义验证（没有对比参考）
+ 无随机SQL生成（Fuzzing）能力

**解决方案**:

1. **完成 sqlancer SQLite 差异测试**（P0，2-3人天）
   + 用 SQLite 作为「正确参考」实现
   + sqlrustgo + SQLite 执行同一 SQL，比较结果集
   + 已有: `DdlGenerator`, `DmlGenerator`, `TlpOracle`
   + 缺失: SQLite adapter, 结果集比较器

2. **激活 sql_corpus 回归测试**（P0，0.5人天）
   + 利用已有的 103 个 SQL 文件
   + 分级执行: fast(<5s) / medium(<30s) / full(<5min)
   + 覆盖: DDL/DML/EXPRESSIONS/FUNCTIONS/TCL/TRANSACTION 等 16 类

3. **Beta gate 集成**（P0）
   + `check_beta_gate.sh` 增加 B10 (sqlancer) 和 B11 (sql_corpus)

**验证**: sql_corpus/ 中所有 SQL 文件在 sqlrustgo 上执行不 panic

**完成判据**: sqlancer 可运行 + sql_corpus fast 全部 PASS + Beta gate 含 B10/B11
### [V310-14] SQLLogicTest 集成（590万用例基线）

**ISSUE**: #3373

**目标**: 在 Beta 阶段建立 SQLLogicTest (SLT) 测试框架，直接利用 SQLite 官方的 623 个测试文件（约 590 万用例）建立 SQL 兼容性基线。

**现状**:
+ SLT 是 SQLite 官方核心测试套件，DuckDB/ClickHouse 等现代数据库均采用
+ 623 个 .test 文件公开可用，覆盖 JOIN/窗口函数/聚合/子查询/表达式求值等全场景
+ 比自研 sqlancer 更快速获得大量验证

**解决方案**:

1. **下载 SLT 用例集**（P0，1人天）
   ```bash
   wget https://www.sqlite.org/src/tarball/sqlite.tar.gz?r=release
   tar xzf sqlite.tar.gz
   mv sqlite/test/sqllogictest crates/sqllogictest/testdata/
   ```

2. **实现 SLT Runner**（P0，3-5人天）
   + `crates/sqllogictest/` crate：解析 .test 文件格式
   + 对每条 SQL 同时调用 sqlrustgo 和 SQLite 执行器
   + 结果比较（浮点数容差支持）

3. **Baseline 报告**（P1，1人天）
   + 输出 `docs/releases/v3.10.0/sqllogictest-baseline/` 基线报告
   + 记录每个 .test 文件的 pass/fail/skip 数

**子任务**:

- [x] [V310-14a] SLT Runner 实现 ✅ — sqllogictest-rs risinglightdb v0.29
- [~] [V310-14b] SLT 用例集下载 — 部分完成: 23个文件 (risinglightdb 8 + DuckDB 7 + prior 9)
  - risinglightdb/sqllogictest-rs 测试套: 8 个 .slt 文件 ✅
  - DuckDB sample tests: 7 个 .test 文件 ✅
  - SQLite 官方套件: 下载受阻于网络 (14MB tarball 超时)
- [ ] [V310-14c] Baseline 报告生成 — 1人天

**验证**: `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata`

**Baseline (Beta, 2026-07-13)**: 1/16 files pass (6.3%)
- PASS: `delete__test_delete.test`
- FAILs reflect sqlrustgo SQL coverage gaps (DuckDB-specific syntax: SEQUENCE, PREPARE, nextval, OFFSET, etc.)

**完成判据**: SLT 套件完整执行 + Beta gate B12 PASS





---

## 3. Issue 创建清单（待提交到 Gitea）

| Gitea Issue | 标题 | 主题标签 |
|-------------|------|---------|
| `#3xxx` | [V310-MASTER] v3.10.0 总控 | P0, milestone-v3.10.0 |
| `#3xxx+1` | [V310-01] DML 完整性 | P0, dml |
| `#3xxx+2` | [V310-02] UNION 集合操作 | P0, sql |
| `#3xxx+3` | [V310-03] ACID 事务正确性 | P0, transaction |
| `#3xxx+4` | [V310-04] ALTER TABLE 完整性 | P0, ddl |
| `#3xxx+5` | [V310-05] 真实崩溃恢复 + 24h SOAK | P0, stability |
| `#3xxx+6` | [V310-06] Wired-SOAK DDL 修复 (PR1) | P0, wire |
| `#3xxx+7` | [V310-07] Catalog 4 层重构 (PR2) | P0, catalog |
| `#3xxx+8` | [V310-08] DDL 执行路径实现 (PR3) | P0, ddl |
| `#3xxx+9` | [V310-09] Wire 协议握手修复 (PR4) | P0, wire |
| `#3xxx+10` | [V310-10] 覆盖率提升至 ≥80% | P1, coverage |
| `#3xxx+11` | [V310-11] TPC-H SF=1 22/22 闭环 | P1, tpch |
| `#3xxx+12` | [V310-12] 其他 ignore 测试 + 跨版本债 | P2, debt |

> **注**: 由于 Gitea 当前无法访问外网，且工作站无法访问 GitHub，具体的 issue 编号需在 Gitea 恢复后由人工创建。本文档作为**完整定义**,准备好后可一键批量创建。

---

## 4. 阶段计划

| 阶段 | 周期 | 核心 ISSUE | 退出判据 |
|------|------|-----------|---------|
| **Alpha** | 4 周 | V310-01, 02, 03, 04, 06 | DML + ACID + ALTER + Wire-DDL 完成 |
| **Beta** | 4 周 | V310-05, 07, 08, 10, 13, 14 | Crash + 24h SOAK + Catalog + 覆盖率 + 测试体系 + SQLLogicTest |
| **RC1** | 2 周 | V310-09 | Wire 协议修复 + sysbench 接入 |
| **RC2-RC8** | 6 周 | V310-11, 12 | TPC-H SF=1 闭环 + P2 债 |
| **GA** | 4 周 | 综合验证 | G1-G10 全 PASS, 168h SOAK PASS |

**总周期**: ~20 周 (5 个月)

---

## 5. 关联资源

- **总控 ISSUE**: 待 Gitea 创建
- **开发计划**: [`V310_DEVELOPMENT_PLAN.md`](V310_DEVELOPMENT_PLAN.md)
- **CLI 计划**: [`V310_CLI_BINARY_PLAN.md`](V310_CLI_BINARY_PLAN.md)
- **ADR-013**: [`../../governance/adr/ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md`](../../../governance/adr/ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md)
- **v3.9.0 评估**: [`../v3.9.0/V390_COMPREHENSIVE_ASSESSMENT.md`](../../v3.9.0/V390_COMPREHENSIVE_ASSESSMENT.md)
- **ARCHITECTURE**: [`../ARCHITECTURE.md`](../ARCHITECTURE.md)
- **STAGE**: [`../STAGE.yaml`](../STAGE.yaml)

---

*本计划由 Claude Code 在 develop/v3.10.0 分支上创建*
*最后更新: 2026-07-11*
