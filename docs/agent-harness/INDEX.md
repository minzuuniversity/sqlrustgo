# Agent Harness — Task → Source/Code/Doc Map

> **目标**: 让任意 Agent 根据任务类型快速找到最小必要上下文, 不读整个仓库。

## 任务 → 主入口

| 任务类型 | 源码入口 | 测试入口 | 文档 |
|---|---|---|---|
| **SQL Parser (语法)** | `crates/parser/src/{lexer,parser,token}.rs` | `crates/parser/tests/`, `tests/integration/sql/parser_*` | `docs/architecture/SQL_SEMANTIC_AUDIT.md` |
| **Binder / Catalog** | `crates/catalog/src/`, `src/system_tables.rs` | `tests/integration/catalog/` | `docs/architecture/EXECUTION_ARCHITECTURE.md` |
| **Planner / CBO** | `crates/planner/src/`, `crates/optimizer/src/`, `src/planner.rs`, `src/cbo_estimator.rs` | `tests/integration/optimizer/` | `docs/architecture/CASCADES_OPTIMIZER.md`, `docs/architecture/cascades_optimizer_design.md` |
| **Executor / Vector** | `crates/executor/src/`, `src/engine_*.rs` | `tests/integration/executor/` | `docs/architecture/EXECUTION_PATH.md`, `docs/architecture/EXECUTION_ARCHITECTURE.md` |
| **Storage / BufferPool** | `crates/storage/src/{file_storage,buffer_pool,bplus_tree,page}.rs` | `crates/storage/tests/`, `tests/unit/file_storage_test.rs` | `docs/architecture/MODULE_LIFECYCLE.md` (test-only 标注) |
| **Transaction / MVCC** | `crates/transaction/src/` | `tests/integration/transaction/` | `docs/governance/wal/{WAL_CONTRACT,TX_LIFECYCLE_SPEC}.md` |
| **WAL / Recovery** | `crates/transaction/src/wal*`, `crates/storage/src/wal_legacy.rs` | `tests/integration/fault_injection/torn_write_recovery.rs` | `docs/governance/wal/WAL_CONTRACT.md` |
| **DDL (CREATE/ALTER/DROP)** | `src/engine_ddl.rs`, `crates/parser/src/parser.rs` (DDL 部分) | `tests/integration/sql/ddl_*` | `docs/architecture/SEMANTIC_DRIVEN_DEVELOPMENT.md` |
| **DML (INSERT/UPDATE/DELETE)** | `src/engine_dml.rs`, `crates/executor/src/{local_executor_dml,mutation_compiler}.rs` | `tests/integration/dml/` | 同上 |
| **SELECT (含 JOIN/AGG/Window)** | `src/engine_select.rs`, `crates/executor/src/{join,window_executor,parallel_group_by}.rs` | `tests/integration/sql/select_*`, `tests/integration/operators/` | `docs/architecture/EXECUTION_PATH.md` |
| **Index / 索引** | `crates/storage/src/{bin_index,adaptive_hash_index}.rs` | `tests/integration/sql/index_*` | (无单文件, 见 architecture/) |
| **MySQL 协议 / Server** | `crates/{mysql-server,mysql-client}/src/`, `crates/server/src/` | `tests/e2e/`, `tests/integration/mysql_*` | `docs/runbooks/...`, `docs/SYSBENCH_WIRE_PROTOCOL_FIX_GUIDE.md` |
| **RAG / 向量检索** | `crates/{rag,vector}/src/` | `crates/{rag,vector}/tests/` | `docs/architecture/v400_graph_storage.md` |
| **图 / GMP** | `crates/{graph,gmp,evidence-graph}/src/` | `crates/gmp/tests/` | `docs/architecture/v400_graph_storage.md` |
| **Crash Recovery** | `crates/transaction/src/recovery*`, `src/savepoint_wiring.rs` | `tests/integration/fault_injection/`, `tests/gate/` | `docs/governance/wal/WAL_CONTRACT.md` |
| **性能 / EXPLAIN** | `crates/executor/src/explain.rs`, `src/execution_engine.rs` | `tests/integration/sql/explain_*` | (PR3 ahi_promoted, PR-A.5 pages_visited) |
| **回归 / Bug Review** | 用 `git log --oneline -- <path>` + GitNexus upstream | 看 issue #XXXX 关联 PR | `docs/closes/`, `reports/` |
| **发布 / Gate 验证** | (无源码) | `tests/gate/`, `scripts/gate/`, `.gitea/workflows/` | `docs/releases/v4.1.0/STAGE.yaml`, `docs/governance/{GATE_CI_CD,GATE_CONDITIONS}.md` |

## 跨任务通用流程

1. **锁定**: `git rev-parse HEAD` + `git status --short` + 当前分支
2. **找文档**: 看 [`DOCUMENTATION_INDEX.md`](DOCUMENTATION_INDEX.md) 找相关 ADR / 设计文档
3. **查符号**: GitNexus `gitnexus_query({query: "<概念>"})` 或 grep
4. **读最小源码**: 从任务入口路径开始
5. **看测试**: 同 crate 的 `tests/` + `tests/integration/` 对应子目录
6. **影响分析**: `gitnexus_impact({target: "<funcName>", direction: "upstream"})`
7. **写测试**: RED → 实现 GREEN → 重构
8. **验证**: clippy + fmt + 相关 gate + cargo test
9. **报告**: 含 5 字段 evidence + commit SHA + 测试输出

## 工具降级链

```
GitNexus MCP → gitnexus CLI (npx gitnexus query) → ripgrep + rust-analyzer
AOCI MCP    → aoci.cli (本地二进制)              → grep + 读 aoci.code.txt
cargo       → cargo metadata / cargo doc --no-deps
GitNexus FTS 失效时 → 用符号图 + 源码 grep
AOCI 不可用时   → 用 docs/governance/ + 源码
```