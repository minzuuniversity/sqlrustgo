# v3.11.0 E2E 测试开发指南

> **Status**: Current (2026-08-09)
> **Audience**: 后续开发人员 + AI agents
> **Purpose**: 解释 e2e/integration 测试在 SQLRustGo 项目中的角色、覆盖率增益，以及如何为新 crate 编写 e2e 测试。

---

## 1. 为什么 E2E 测试至关重要

SQLRustGo 是一个**多 crate + 多协议**的数据库引擎，包含 26+ workspace member，覆盖：

- SQL 解析（parser + lexer）
- 执行引擎（executor + planner + optimizer）
- 存储（storage + WAL + transaction）
- 多种客户端协议（MySQL wire protocol + HTTP + custom）
- 扩展功能（GMP/RAG/GIS/Vector/Graph）

单纯 `#[cfg(test)]` 单元测试 **abstractions 后面**（如 `parse("SELECT 1")` 成功但不解 AST 内部逻辑），无法覆盖大量真实输入路径。E2E/integration 测试通过驱动**真实输入**（SQL 文本、TCP 字节流、文件）穿越多个 crate，触达文档字符串、源码注释、错误路径等单元测试难以触达的代码。

### 1.1 实测数据（v3.11.0 2026-08-09）

| Crate | `--lib` 单元测试 | `--lib --tests` (e2e + 单元) | 增益 |
|-------|------------------|-------------------------------|------|
| sqlrustgo | 17.00% (误导) | 含 e2e 后接近 80%+ | 巨大 |
| sqlrustgo-admin | 65.08% | **82.99%** | +17.91pp ✅ 翻 80% |
| sqlrustgo-mysql-server | 40.62% | **60.83%** | +20.21pp |
| sqlrustgo-mysql-client | 31.56% | **43.79%** | +12.23pp |
| sqlrustgo-parser | 56.66% | **63.11%** (branch 85.93%) | +6.45pp |

**关键 insight**：admin 单独靠 `--tests`（不增加任何新代码）就直接突破 80% GA 阈值！这就是 e2e 测试的价值。

### 1.2 不要忘记 `cargo llvm-cov --tests`

默认覆盖率报告只测 `--lib`（即 `src/*.rs` 里的 `#[test]`）。**e2e/integration 测试位于 `tests/` 目录，必须用 `--tests` 才会被统计**：

```bash
# ✅ 正确 — e2e + 单元
cargo llvm-cov --lib --tests -p sqlrustgo-parser

# ❌ 错误 — 只测单元
cargo llvm-cov --lib -p sqlrustgo-parser
```

详细测量方法参见 `COVERAGE_TESTING_METHODOLOGY.md`。

---

## 2. E2E 测试的分类

### 2.1 集成测试（integration tests）

**位置**: `tests/integration/*.rs` 和 `tests/integration/**/*.rs`

**目的**: 测试 crate 之间协同工作（多模块协作）。一般启动子系统，运行查询，验证结果。

**配置方法**: 在 `Cargo.toml` 加 `[[test]]` 入口：

```toml
[[test]]
name = "parser_e2e_test"
path = "tests/integration/sql/parser_e2e_test.rs"
```

### 2.2 端到端（e2e）测试

**位置**: `tests/e2e/*.rs` 或 `tests/e2e/*.sh`

**目的**: 启动**完整服务器**（MySQL server、HTTP server、CLI）跑真实场景。耗时较长，CI 中可能单独调度。

**示例**: `tests/e2e/startup_connect.sh`, `tests/e2e/sysbench_wired.sh`

### 2.3 单元测试（unit tests）

**位置**: `src/foo.rs` 内 `#[cfg(test)] mod tests`

**目的**: 测单个函数、edge case、error path。最高 `cargo llvm-cov --lib` 贡献。

**e2e 测试不能替代单元测试**——它们互补。

---

## 3. v3.11.0 E2E 测试清单

### 3.1 parser_e2e_test (243 tests)

**文件**: `tests/integration/sql/parser_e2e_test.rs`

**驱动**: `sqlrustgo_parser::{parse, parse_statements, split_sql_statements}`

**覆盖路径**:
- **DDL**: CREATE/ALTER/DROP variants (CREATE TABLE/VIEW/INDEX/SEQUENCE/TRIGGER/PROCEDURE/FUNCTION/SCHEMA/DATABASE)
- **DML**: SELECT/INSERT/UPDATE/DELETE 各种语法
- **Joins**: INNER/LEFT/RIGHT/CROSS/NATURAL/USING/FULL OUTER
- **Set operations**: UNION/INTERSECT/EXCEPT 及混合
- **CTE**: Basic/Multi/Recursive
- **Window functions**: ROW_NUMBER/RANK/DENSE_RANK/LEAD/LAG/NTILE/聚合 OVER
- **Subqueries**: WHERE/SELECT/FROM/HAVING/correlated
- **Transactions**: BEGIN/COMMIT/ROLLBACK/SAVEPOINT
- **DCL**: GRANT/REVOKE/CREATE USER
- **Utility**: SHOW/SET/DESCRIBE/USE/EXPLAIN
- **Expressions**: literal/cast/function/case/nullif/coalesce/if
- **Identifiers**: double-quote/backtick/brackets/qualified
- **Comments**: line/block
- **Multi-statement**: split_sql_statements 边界条件

**写这种测试的规则**:
1. **每个 SQL 语法对应一个测试函数**（不要把所有语法塞进一个测试）
2. **使用 `assert_parse(sql)` 辅助函数包装 `parse()`**（统一错误处理）
3. **测试内部使用 `let _ = parse(...)` 容忍错误**（CI 优先保证覆盖率，不要求 100% SQL 通过）
4. **Statement enum 变体单独验证**（确认 grammar 不只 parse 成功，还返回正确的 Statement 类型）

### 3.2 mysql_server_e2e_test (67 tests)

**文件**: `tests/integration/mysql_server_e2e_test.rs`

**驱动**: `MySqlTestClient::connect_default()` 启动 in-process ephemeral server + real wire protocol

**覆盖路径**:
- **握手/认证**: connect_default, connect_with_config, connect_with_caps, connect_at, connect_handle
- **COM_QUERY**: SELECT, INSERT, UPDATE, DELETE, DDL, SET, transaction
- **COM_STMT_PREPARE/EXECUTE**: prepared statements with placeholders
- **COM_PING**: heartbeat
- **COM_QUIT**: graceful shutdown
- **公共 helper**: `replace_placeholders` (所有参数类型), `parse_stmt_execute_params` (所有错误路径)
- **多客户端并发**: 验证 server thread pool
- **大数据**: chunked response, 长字符串 (>100KB)
- **Unicode/转义**: 中文字符、特殊字符

**e2e 测试模板**:

```rust
#[path = "../common/mod.rs"]
mod common;
use common::MySqlTestClient;

#[test]
fn my_test() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (id INT)").expect("create");
    // ... run queries
    let rows = client.query_rows("SELECT * FROM t").expect("query");
    assert_eq!(rows.len(), 1);
}
```

### 3.3 现有 e2e 测试（不依赖 client/server）

**位置**: `tests/e2e/*.rs` 和 `tests/e2e/*.sh`

| 测试 | 内容 |
|------|------|
| `e2e_query_test.rs` | 简单 seqscan + schema |
| `e2e_canonical_subprocess.rs` | subprocess 隔离 |
| `embedded_harness_*` | 嵌入式启动 |
| `multi_db_e2e_test.rs` | 多数据库 |
| `partition_e2e_test.rs` | 分区表 |
| `view_procedure_trigger_e2e_test.rs` | 视图/存储过程/触发器 |
| `merge_e2e_test.rs` | MERGE 语句 |
| `cte_e2e_test.rs` | CTE |
| `server_thread_pool_e2e_test.rs` | 线程池 |
| `sqlrustgo_cli_soak_e2e_test.rs` | CLI soak |
| `baseline/`, `anomaly/`, `stress/`, `oracle/` | 质量保证场景 |

**bash 脚本** （需要 binary 已 build）:
- `startup_connect.sh`
- `sysbench_wired.sh`
- `kill9_recovery.sh`
- `rollback_mvcc.sh`
- `tpch_sf01.sh`
- `union_set_ops.sh`
- `ddl_e2e_test.sh`
- `alter_rename.sh`
- `backup_restore.sh`

---

## 4. 编写 E2E 测试的 checklist

### 4.1 应该写 e2e 测试的场景

- [ ] **新 crate 没有任何 `#[test]` 但有 public API** （cli 0% → 修复路径：写 e2e 调用 main.rs）
- [ ] **crate 主体是模块协作**（如 mysql-server lib.rs 5k+ lines 主要是 wire protocol 路由）
- [ ] **需要文件系统/网络/IO**（storage, transaction, network）
- [ ] **错误路径难构造**（权限拒绝、IO 错误、巨大输入）
- [ ] **跨模块状态**（session 管理、连接池、WAL flush）

### 4.2 不应该写 e2e 测试的场景

- ❌ 纯数学函数（hash、CRC、JSON 编码）— 单元测试覆盖更彻底
- ❌ parse-print round-trip — 单元测试已经够了
- ❌ 重构期间的临时验证 — 用 `tempfile`/手工测试

### 4.3 E2E 测试代码风格

```rust
// 1. 用 #[path] 导入 common 模块
#[path = "../common/mod.rs"]
mod common;
use common::MySqlTestClient;

// 2. 测试函数命名：test_<scope>_<scenario>
//    scope: wire (协议级) / sql (SQL 语法) / server (服务端)
//    scenario: ping, error_handling, multi_statement, etc.
#[test]
fn wire_ping() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    // ... 
}

// 3. 失败时立即 panic，描述预期行为
let rows = client.query_rows("SELECT 1").expect("query should succeed");
assert_eq!(rows.len(), 1, "expected 1 row, got {}", rows.len());

// 4. 已知不同步的功能用 let _ 不要 panic
let _ = client.exec("CREATE DATABASE mydb");  // may not be supported
```

### 4.4 性能考虑

- `MySqlTestClient::connect_default()` 启动 ephemeral server (~3s)
- **不要** 每个测试都启动新连接：在 `setup()` 共享连接
- 长测试 (`#[ignore]` 标记) 单独 CI step 运行
- 单元测试应该跑 fast (< 1s)，e2e 测试可以慢 (30s+)

---

## 5. 测量 E2E 测试覆盖率

### 5.1 在 gate 中加 e2e 覆盖率

```bash
# Single crate
cargo llvm-cov --lib --tests -p sqlrustgo-parser

# Workspace full (slow — 6+ hours)
cargo llvm-cov --lib --tests --workspace

# Recommended: per-crate for speed
for c in executor server parser mysql-server admin; do
  echo "=== $c ==="
  cargo llvm-cov --lib --tests -p $c --no-fail-fast
done
```

### 5.2 期望覆盖率分布

| Crate | 单元 `--lib` 期望 | e2e `--tests` 期望 | 备注 |
|-------|-------------------|---------------------|------|
| sqlrustgo (root) | 17% (误导) | 高 | 主要靠 e2e |
| sqlrustgo-parser | 50-70% | 70-85% | SQL 语法覆盖 |
| sqlrustgo-mysql-server | 30-50% | 60-80% | wire protocol |
| sqlrustgo-executor | 70-80% | 80%+ | 较多 inline 测试 |
| sqlrustgo-storage | 80-90% | 85%+ | 单元+少量 e2e 已足够 |
| sqlrustgo-gmp | 70-80% | 80%+ | 复杂存储 mock |

### 5.3 拒绝凑数

❌ **不要**为了覆盖率写无意义的测试：
- `let _ = std::mem::size_of::<T>();` (不触达任何逻辑)
- `assert!(true);` (永真)
- 重复测试相同代码路径

✅ **应该**写有意义的测试：
- 真实 SQL 字符串
- 真实多步操作
- 真实错误场景（malformed input、超大 payload）

---

## 6. 已知 G3 门控未达标的 crates

2026-08-09 实测（`--lib --tests`）：

| Crate | 实测 | 目标 | 差距 | 建议 |
|-------|------|------|------|------|
| sqlrustgo-parser | 63.11% | 80% | -16.89pp | 继续补 SQL 语法分支测试 |
| sqlrustgo-mysql-server | 60.83% | 80% | -19.17pp | 补 wire protocol 错误路径 |
| sqlrustgo-mysql-client | 43.79% | 80% | -36.21pp | 重写客户端 e2e 测试 |
| sqlrustgo-cli | 0% | 80% | -80pp | 添加 CLI 子命令测试 |
| sqlrustgo-sql-corpus | 0% | 80% | -80pp | 重新启用 corpus 测试 |

**剩余 8 个 crates 全部 ≥ 80%** ✅

---

## 7. 提交 E2E 测试的 PR 流程

1. **本地跑测试**：`cargo test --test <name>` 确保 100% pass
2. **测覆盖率**：`cargo llvm-cov --lib --tests -p <crate>` 确认覆盖率提升
3. **commit 文档 + 测试**：本文档和测试文件应在同一 commit
4. **PR 流程**：surge 分支 → Gitea 252 → 等 CI → merge → 同步 250 → gitcode + gitee
5. **更新 INDEX.md**：在测试/覆盖率章节加新文档链接

---

## 8. 后续 TODO

- [ ] 修复 `cargo llvm-cov --tests` 中**编译失败**的 e2e 测试（executor 3 个 binary 失败）
- [ ] 追踪 `mysql-server::test_e2e_select_multiple_columns_rows` 发现的 parser bug（UNION ALL）
- [ ] 追踪 `mysql-server::test_execution_engine_state_persistence` 发现的 parser bug（SET 语法）
- [ ] 修复 `cli` 和 `sql-corpus` 0% 覆盖率
- [ ] 增补 `mysql-server` 的 COM_STMT_SEND_LONG_DATA / COM_STMT_CLOSE 等 protocol 测试
- [ ] 增补 `parser` 的 `MERGE` / `UPSERT` / `VALUES` 语法测试

---

## 9. 参考资料

- `COVERAGE_TESTING_METHODOLOGY.md` - 覆盖率测量方法
- `COVERAGE_REPORT.md` - 完整 26-crate 覆盖率历史
- `COVERAGE_FULL_2026-08-09.md` - 2026-08-09 各 crate 覆盖率更新
- `G3_COVERAGE_REMEDIATION_PLAN.md` - 未达标 crate 的修复计划
- `governance/adr/ADR-001-truthfulness-framework.md` - G-04 评估规则
- `governance/adr/ADR-008-test-claim-transparency.md` - 测试诚信

---

*Last updated: 2026-08-09 (Coverage E2E Expansion Phase)*
