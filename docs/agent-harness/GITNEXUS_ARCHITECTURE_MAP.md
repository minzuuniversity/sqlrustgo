# GitNexus Architecture Map

> **基线**: HEAD `583050bc63` (fix/4597)
> **索引状态**: lastCommit=`b1e9a98ee6bac1e863ab8826c54a41500455b71d` (2026-10-08), 落后 HEAD ~1 周
> **来源**: `.gitnexus/meta.json`, FTS 索引因 database version 42 vs build 40 失效, 符号图仍可用

## 索引数据规模

| 维度 | 数值 |
|---|---|
| Files indexed | 5544 |
| Nodes (符号) | 103,833 |
| Edges (关系) | 157,185 |
| Communities (簇) | 1902 |
| Processes (执行流) | 300 |
| Embeddings | 0 (FTS 失效) |

> ⚠️ GitNexus AGENTS.md (line 212) 显示 110,918 symbols / 160,471 relationships — 与 meta.json 不一致, 表明注入的 AGENTS 段落后于 meta.json 实际统计。

## 4 条核心执行链 (按源码核实)

### 1. SQL 查询 (SELECT)

```
Client (MySQL wire) → crates/mysql-server/src/server.rs
  ↓ execute(...)
crates/server/src/lib.rs (TCP/connection lifecycle)
  ↓ crates::executor::SqlExecutor::execute
crates/executor/src/sql_executor.rs (SQL parsing → execution)
  ↓ sqlrustgo_parser::parse
crates/parser/src/{lexer,parser}.rs
  ↓ AST Statement
src/execution_engine_methods.rs::execute_statement (本 crate 主入口)
  ↓ SELECT 分支
src/engine_select.rs (主要 SELECT 路径; 9000+ LOC)
  ↓ LogicalPlan
src/planner.rs + crates/planner/src/ (逻辑计划)
  ↓ PhysicalPlan
crates/optimizer/src/ (CBO + Rule)
  ↓ Executor::execute
crates/executor/src/{executor,pipeline_executor,parallel_executor}.rs
  ↓ scan / fetch
crates/storage/src/{file_storage,bin_index}.rs
  ↓ Result batch
ExecutorResult → wire → Client
```

**关键交叉点**:
- `src/execution_engine.rs::ExecutionEngine::execute()` (根 crate 主入口)
- `src/engine_select.rs` 含 V312-58 Sprint 3-6 诊断导出 (PR3 ahi_promoted 注入点)
- EXPLAIN 在 `crates/executor/src/explain.rs` (被 PR3 + PR-A.5 改造)

### 2. 事务 (Transaction)

```
Client (BEGIN/COMMIT/ROLLBACK)
  ↓ wire
crates/mysql-server/src/server.rs
  ↓ crates::transaction::TxManager
crates/transaction/src/tx_manager.rs (Transaction lifecycle)
  ↓ MVCC version assignment
crates/transaction/src/mvcc*.rs (MVCC)
  ↓ WAL append
crates/transaction/src/wal*.rs (WAL writer)
  ↓ WAL fsync → durability
crates/storage/src/wal_legacy.rs (实际写盘; WalEntry 序列化)
```

**⚠️ 关键不变量 (来自 src/lib.rs:49-63 注释与 #5192 系列 commit)**:
- `WalEntry` 字段: tx_id, entry_type, table_id, key, data, lsn, timestamp, table_name (#5055), database_name (#5192)
- 真正写 WAL 的不是 `WalStorage::log_insert` (死代码), 而是 `FileStorage::wal_row_entry` (审计发现 — #5192 系列 commit)
- 任何修改 WalEntry 字段必须同步更新所有 fixture (tests/integration/migration/wal_integration_test.rs 缺 database_name 是历史症状, 已由 #5261 修)

### 3. 索引 (Index)

```
DDL: CREATE INDEX
  ↓ wire
crates/parser/src/parser.rs (parse CREATE INDEX)
  ↓ ExecutionEngine::execute_statement (DDL 分支)
src/engine_ddl.rs (DDL dispatch)
  ↓ Catalog::create_index
crates/catalog/src/catalog.rs (schema 管理)
  ↓ IndexManager::create
crates/storage/src/{bin_index,adaptive_hash_index}.rs
  ↓ Index storage (B+Tree / AHI)
crates/storage/src/{bplus_tree,page,buffer_pool}.rs (⚠️ test-only for production)

查询: SELECT WHERE indexed_col = ?
  ↓ engine_select.rs::optimize_plan
crates/optimizer/src/ (cost-based: choose index scan)
  ↓ IndexScan Exec
crates/storage/src/bin_index.rs::index_scan
```

**关键不变量**:
- AHI (`adaptive_hash_index.rs`) 提供 `promoted_count()` — PR3 EXPLAIN ahi_promoted 的依据
- BufferPool/Page/BPlusTree 在 src/lib.rs 标注为 test-only — 但 storage 测试 / 基准仍使用

### 4. 系统启动 + 恢复

```
process startup (./sqlrustgo-mysql-server 或 REPL)
  ↓
crates/server/src/lib.rs::start / crates/cli/src/main.rs
  ↓ Catalog load
crates/catalog/src/catalog.rs::load_from_disk
  ↓ Storage init
crates/storage/src/file_storage.rs::new (或 MvccStorage / WalStorage / ParallelWalStorage)
  ↓ WAL replay (如果有 .wal 文件)
crates/transaction/src/wal*.rs::replay
  ↓ ApplyLogEntry → storage.insert (with database_name)
  ↓ recovery complete → "Ready to accept connections"
  ↓ listener.accept → serve clients
```

**关键不变量** (from WAL_CONTRACT.md + TX_LIFECYCLE_SPEC.md):
- WAL replay 必须用 entry 中 `database_name` 而非当前库
- 表名必须有 `table_name`, 否则 replay 必须拒绝 (而非猜测写到错表)

## 影响分析 (上游调用示例)

按 src/lib.rs 的 re-export 链:

```
sqlrustgo_storage::FileStorage
  ↑ src/execution_engine.rs (主要消费方)
  ↑ src/engine_dml.rs (DML)
  ↑ src/savepoint_wiring.rs (savepoint)
  ↑ src/execution_engine_methods.rs
  ↑ src/engine_ddl.rs (DDL)
  ↑ tests/integration/* (100+ 测试文件)
```

修改 `FileStorage::insert` 影响 ~50+ 上游调用方 — 必须 `gitnexus_impact` 验证。

## 已知 GitNexus 失效点 (降级路径)

| 问题 | 影响 | 降级 |
|---|---|---|
| FTS 索引 (db version 42 vs build 40) | `file_fts`, `function_fts`, `class_fts`, `method_fts`, `interface_fts` 索引重建失败, 全文搜索不可用 | `rg` + 源码 |
| 索引 commit 落后 ~1 周 | `gitnexus_query` 可能漏最新 symbol | 配合 `git log --diff-filter=A --name-only HEAD~50..HEAD` |
| GitNexus AGENTS.md 注入的 stats 与 meta.json 不一致 | 误导 | 直接读 meta.json |

## 待办

- [ ] `npx gitnexus analyze` 重建 FTS 与重新对齐 HEAD
- [ ] 把 `GITNEXUS_ARCHITECTURE_MAP.md` 加入 agent-harness CI 检查, 索引版本落后 > 14 天报警