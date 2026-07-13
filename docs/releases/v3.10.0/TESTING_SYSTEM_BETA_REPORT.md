# SQLRustGo v3.10.0 — Beta 测试体系建立报告

**版本：** v3.10.0  
**阶段：** Beta  
**作者：** Claude Code (DeepSeek Feedback)  
**日期：** 2026-07-13  
**ISSUE：** #3274（见本文档末尾）

---

## 1. 背景

### 1.1 Phase 2 重构后的测试现状

Phase 2（v3.10.0）将 224 个根目录测试文件迁移至 13 个子目录，并建立了 335 个 `[[test]]` Cargo.toml 条目。重构验证表明：

- **396 个测试目标**注册在 cargo metadata 中
- **341 个测试文件**分布在 13 个子目录
- **13 个代表测试全部通过**（146+ 测试用例）
- 但整体测试体系仍存在结构性缺陷

### 1.2 现有基础设施盘点

| 资源 | 状态 | 说明 |
|------|------|------|
| `crates/sqlancer` | **骨架** (~100行) | SQL fuzzing 框架，DDL/DML 生成器 + TLP Oracle |
| `crates/test-runner` | **骨架** | async 多进程测试编排 |
| `crates/test-registry` | **空壳** | 测试注册表 |
| `sql_corpus/` | **103 个 SQL 文件，7071 行** | 覆盖 DDL/DML/EXPRESSIONS/FUNCTIONS 等 16 类 |
| `tools/xtask` | **2 个工具** | architecture-check, dead-modules |

**问题：** 大量基础设施存在但从未被使用。103 个 SQL 文件躺在 `sql_corpus/` 里零成本就能激活。

---

## 2. 当前测试体系的问题诊断

### 2.1 骨架工具空转（技术债）

`sqlancer`（SQL differential testing）、`test-runner`（并行测试编排）、`test-registry`（测试注册表）三个 crate 都是骨架或空壳。已有的资源没有产生价值。

### 2.2 缺乏跨数据库语义验证

现有测试只验证"SQL 语句能执行"，不验证"执行结果是否正确"。没有对比参考实现。

### 2.3 无随机 SQL 生成（Fuzzing）

现有测试是手写 SQL 的固定覆盖，存在大量未触达的代码路径。随机 SQL 生成可以发现边缘情况。

### 2.4 测试分层不清晰

所有测试在同一个抽象层级：有的纯内存单元测试（毫秒级），有的真实进程 E2E（分钟级），但没有按执行成本分层并相应分配到不同 CI 门禁。

---

## 3. 开源数据库测试系统对比

| 系统 | 特点 | 能否引入 |
|------|------|---------|
| **SQLite** | SQL 参考实现，确定性，C 代码成熟 | **最佳参考** |
| **PostgreSQL** | SQL 标准兼容好 | ⚠️ 重量级 |
| **DuckDB** | 分析型 SQL，嵌入式 | ⚠️ 依赖复杂 |
| **sqlancer** | differential testing（自研，已引入骨架） | 应完成实现 |
| **SQLSmith** | 结构化 SQL fuzzing | 可参考 |
| **AFL / libFuzzer** | 覆盖率引导 fuzzing | 可集成 |

**结论：** SQLite 是最佳参考实现——完全兼容 SQL-92/99、C 代码成熟、无外部依赖、每个函数行为确定性。

---

## 4. Beta 阶段改进方案

### 方案 A：完成 `sqlancer` 实现（SQLite 差异测试）⭐⭐⭐

**核心思路：** 用 SQLite 作为"正确参考"，用 sqlrustgo 执行同一 SQL，比较结果集。

```
SQL Generator ──▶ sqlrustgo 执行 ──▶ 结果集 A
                ──▶ SQLite 执行 ──▶ 结果集 B
                              ▼
                    compare(A, B) → 一致 PASS / 不一致 BUG
```

**已有基础：**
- `DdlGenerator`：生成 CREATE/DROP TABLE
- `DmlGenerator`：生成 INSERT/SELECT
- `TlpOracle`：三路逻辑编程 Oracle

**缺失部分：**
- SQLite adapter（约 100 行）
- 结果集比较器（约 150 行）
- 参考数据库执行器（约 100 行）
- DML 生成扩展（JOIN、聚合、子查询）

**工作量估算：** 约 350-500 行 Rust，2-3 人天

**验收标准：**
- `cargo test -p sqlancer` 至少覆盖 50 个随机生成的 SQL
- 每个随机 SQL 同时在 sqlrustgo 和 SQLite 上执行
- 结果不一致时输出详细诊断信息

### 方案 B：激活 `sql_corpus` 回归测试套件 ⭐⭐⭐

**核心思路：** 已有 103 个 SQL 文件，7071 行，覆盖 16 个类别，从未作为测试运行。

**分级执行：**

| 级别 | 覆盖 | 执行时间 | 说明 |
|------|------|---------|------|
| `fast` | DDL + 简单 DML | < 5s | 每次 commit 运行 |
| `medium` | DML + 表达式 | < 30s | PR gate 运行 |
| `full` | TCL + 事务 + 触发器 | < 5min | Beta gate 运行 |

**已有资源：**
- `sql_corpus/DDL/` — ALTER_TABLE, CREATE_TABLE, DROP_TABLE 等
- `sql_corpus/DML/` — INSERT, UPDATE, DELETE
- `sql_corpus/EXPRESSIONS/` — case、datetime、math、logical 等
- `sql_corpus/FUNCTIONS/` — 聚合函数、字符串函数
- `sql_corpus/TCL/` — COMMIT, ROLLBACK, SAVEPOINT
- `sql_corpus/TRANSACTION/` — 多语句事务
- `sql_corpus/TRIGGERS/` — CREATE TRIGGER

**验收标准：**
- `sql_corpus/` 中所有 SQL 文件在 sqlrustgo 上执行不 panic
- 分类报告：`sql_corpus/DDL/`: N passed, M failed

### 方案 C：测试分层金字塔 ⭐⭐

**目标：** 给每个测试标注执行层，让 CI 快速失败在低层级。

```
                    E2E (TCP)        10 tests, 10min  — 最终验证
                   Integration       100 tests, 2min   — 语义正确性
                   Memory Unit       200 tests, 30s    — API 正确性
                   Oracle/Diff       ∞ 随机, 不限时间  — 差异测试
```

**实现方式：**

在 `Cargo.toml` 的 `[[test]]` 条目中添加 `test-level` metadata（未来扩展）：

```toml
[[test]]
name = "dml_integration_test"
path = "tests/integration/dml_integration_test.rs"
test-level = "memory"    # memory | integration | e2e | oracle
```

在 `check_beta_gate.sh` 中增加按级别执行：

```bash
# B-TEST-LEVELS: 分层测试门禁
check "B_TEST_MEMORY" "cargo test --lib 2>/dev/null" true
check "B_TEST_INTEGRATION" "bash scripts/test_levels.sh --fast" false
check "B_TEST_ORACLE" "cargo test -p sqlancer -- --test-threads=4" true
```

### 方案 D：集成 `sqlancer` 到 Beta 门禁 ⭐⭐

**目标：** 将随机差异测试加入 Beta 门禁，在发布前发现语义错误。

在 `check_beta_gate.sh` 中增加：

```bash
# B9_ORACLE_SQLANCER: SQLite 差异测试
check "B9_ORACLE_SQLANCER" \
    "cargo test -p sqlancer 2>/dev/null || echo 'sqlancer not yet implemented'" \
    true   # WARN OK in Beta
```

**验收标准：**
- Beta 阶段：WARN-only，不阻塞发布
- GA 阶段：FAIL 阻塞发布

---

## 5. Beta 阶段实施计划

### 5.1 Issue 定义

**ISSUE #3274：** Beta 测试体系建立（见本文档末尾）

### 5.2 任务分解

| 任务 | 优先级 | 工作量 | 验收条件 |
|------|--------|--------|---------|
| T1: SQLite adapter + 结果比较器 | P0 | 2 人天 | sqlancer 可同时在 sqlrustgo + SQLite 执行 |
| T2: 激活 sql_corpus 快速回归 | P0 | 0.5 人天 | DDL/DML 目录 SQL 全部通过 |
| T3: sql_corpus 分类测试报告 | P1 | 0.5 人天 | HTML 报告输出到 /tmp/corpus_report/ |
| T4: DML 生成扩展（JOIN/聚合） | P1 | 1 人天 | 支持 GROUP BY + SUM/COUNT/AVG |
| T5: 分层测试标记系统 | P2 | 1 人天 | Cargo.toml metadata 标注完成 |
| T6: Beta gate 集成 | P0 | 0.5 人天 | `check_beta_gate.sh` 含 B9_ORACLE_SQLANCER |
| T7: GA gate 集成 | P1 | 0.5 人天 | `check_rc_gate_v3.10.0.sh` 含差异测试 |

### 5.3 Beta 门禁新增检查

在 `check_beta_gate.sh` 中增加：

```bash
# B9: Oracle / Differential Testing
check "B9_ORACLE_SQLANCER" \
    "cargo test -p sqlancer 2>/dev/null || echo 'SKIP'" \
    true   # WARN OK — sqlancer in development

# B10: sql_corpus Fast Regression
check "B10_SQL_CORPUS_FAST" \
    "bash scripts/test_sql_corpus.sh --fast 2>/dev/null || echo 'SKIP'" \
    true   # WARN OK — corpus not yet integrated
```

---

## 6. 测试分层设计（参考）

```
┌──────────────────────────────────────────────────────────────┐
│                    测试金字塔                                    │
│                                                              │
│  L4 ORACLE    sqlancer (SQLite diff)   ∞ random, 10min+    │
│  L3 E2E       TCP server + client      ~20 tests, 5min     │
│  L2 INTEGR    Multi-component          ~100 tests, 2min     │
│  L1 MEMORY    Unit / API               ~300 tests, 30s      │
│                                                              │
└──────────────────────────────────────────────────────────

每层独立门禁：
  L1: 每次 commit 自动运行
  L2: PR gate 自动运行
  L3: Beta gate 手动/定时运行
  L4: GA gate 必须通过
```

---

## 7. 验收标准

### Beta 阶段目标

- [ ] `crates/sqlancer` 有完整 SQLite adapter + 结果比较器
- [ ] `cargo test -p sqlancer` 可运行，输出差异测试结果
- [ ] `sql_corpus/DDL/` 中所有 SQL 在 sqlrustgo 上执行不 panic
- [ ] `check_beta_gate.sh` 包含 B9_ORACLE_SQLANCER 检查
- [ ] 本 ISSUE 所有任务关闭

### GA 阶段目标

- [ ] sqlancer 支持 JOIN、聚合、子查询的差异测试
- [ ] `sql_corpus/` 全部分类测试通过
- [ ] `check_rc_gate_v3.10.0.sh` 包含 L4 Oracle 门禁
- [ ] 测试覆盖率报告可导出

---

## 8. 技术细节

### 8.1 SQLite Adapter 设计

```rust
// crates/sqlancer/src/adapters/sqlite.rs
use rusqlite::Connection;

pub struct SqliteAdapter {
    conn: Connection,
}

impl SqliteAdapter {
    pub fn new() -> Result<Self, SqliteError> {
        let conn = Connection::open_in_memory()?;
        Ok(Self { conn })
    }

    pub fn execute(&mut self, sql: &str) -> Result<QueryResult, SqliteError> {
        let mut stmt = self.conn.prepare(sql)?;
        let cols = stmt.column_names();
        let rows: Vec<Vec<Value>> = stmt
            .query_map([], |row| {
                Ok((0..row.column_count())
                    .map(|i| row.get(i))
                    .collect::<Vec<_>>())
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(QueryResult { cols: cols.to_vec(), rows })
    }
}
```

### 8.2 结果集比较策略

```rust
pub fn compare(a: &QueryResult, b: &QueryResult) -> DiffResult {
    // 1. 行数比较
    if a.rows.len() != b.rows.len() {
        return DiffResult::RowCountMismatch(a.rows.len(), b.rows.len());
    }
    // 2. 每行比较（浮点数用近似比较）
    for (ra, rb) in a.rows.iter().zip(b.rows.iter()) {
        if !approx_eq_rows(ra, rb) {
            return DiffResult::ValueMismatch(ra.clone(), rb.clone());
        }
    }
    DiffResult::Identical
}
```

---

## 9. 参考资料

- [SQLancer 项目](https://github.com/sqlancer/sqlancer)
- [SQLSmith 论文](https://www.cs.purdue.edu/~szhang/papers/SQLSmith.pdf)
- [DuckDB Differential Testing](https://duckdb.org/2021/10/differential-testing.html)
- [SQLite 测试方法](https://www.sqlite.org/testing.html)
- Issue #3274: Beta 测试体系建立

---

## ISSUE #3274

```json
{
  "title": "[Beta] 建立全面测试体系：sqlancer 差异测试 + sql_corpus 回归",
  "body": "## 问题描述\n\nPhase 2（v3.10.0）重构后测试目录已重组为 13 个子目录、341 个文件，但测试体系仍存在以下问题：\n\n1. **骨架工具空转**：crates/sqlancer（~100行骨架）、crates/test-runner（空壳）、crates/test-registry（空壳）从未被使用\n2. **缺乏语义验证**：现有测试只验证「SQL 能执行」，不验证「结果正确」\n3. **sql_corpus 未激活**：103 个 SQL 文件（7071 行）躺在 sql_corpus/ 目录中，从未作为测试运行\n4. **无随机 SQL 生成**：没有 fuzzing 能力，无法发现边缘情况\n\n## 解决方案\n\n详见 docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md\n\n### 核心方案\n\n1. **完成 sqlancer SQLite 差异测试**（P0）：用 SQLite 作为正确参考，sqlrustgo 执行同一 SQL，比较结果集\n2. **激活 sql_corpus 回归测试**（P0）：利用已有 103 个 SQL 文件建立回归测试套件\n3. **Beta gate 集成**（P0）：在 check_beta_gate.sh 中增加 B9_ORACLE_SQLANCER 和 B10_SQL_CORPUS_FAST\n\n## 验收标准\n\n- [ ] crates/sqlancer 有完整 SQLite adapter + 结果比较器\n- [ ] cargo test -p sqlancer 可运行，输出差异测试结果\n- [ ] sql_corpus/DDL/ 中所有 SQL 在 sqlrustgo 上执行不 panic\n- [ ] check_beta_gate.sh 包含 B9_ORACLE_SQLANCER 检查\n\n## 相关文件\n\n- 报告：docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md\n- 门禁：scripts/gate/check_beta_gate.sh\n- sqlancer：crates/sqlancer/\n- sql_corpus：sql_corpus/\n\n## 阶段\n\n**Beta** — v3.10.0 Beta 阶段\n\n## 优先级\n\nP0 / 高"
}
```
