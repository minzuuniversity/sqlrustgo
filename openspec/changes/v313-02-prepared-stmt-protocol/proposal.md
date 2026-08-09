# V313-02: Prepared Statement Protocol 实现

## Why

预处理语句（Prepared Statement）是数据库系统的标准功能，也是 MySQL Wire Protocol 的核心组成部分。客户端通过 `COM_STMT_PREPARE` 发送 SQL 模板，服务器返回语句 ID 和参数元数据；随后客户端通过 `COM_STMT_EXECUTE` 携带参数执行；最后通过 `COM_STMT_CLOSE` 释放资源。这套协议在 JDBC、MySQL Connector/Python 等主流客户端库中广泛使用。

V312-21 的 `prepared_stmt_roundtrip` fixture 揭示了当前实现的缺口：parser 支持 `PREPARE name FROM 'SELECT 1'` 语法，但 executor 层尚未将 `Statement::Prepare` / `Statement::Execute` / `Statement::Deallocate` 接入执行流程，导致返回 `"Parse error: Expected As, got From"`。

本变更实现完整的预处理语句协议支持，补齐 v3.13 MySQL 兼容性的关键缺口。

## What Changes

- `crates/parser/src/parser.rs`：确认并修复合并冲突，确保 `parse_prepare()` / `parse_execute()` / `parse_deallocate()` 语法完整
- `crates/executor/src/logical_execute.rs` 或等价模块：新增 `PreparedStatementCache` 和三种语句的执行入口
- `crates/network/`：若 `COM_STMT_PREPARE` / `COM_STMT_EXECUTE` / `COM_STMT_CLOSE` 包尚未在网络层解码，补齐对应 handler
- `tests/compat/prepared_stmt_protocol.sql` + `.out`：端到端 fixture，验证 PREPARE/EXECUTE/CLOSE 完整流程
- `scripts/gate/run_compat_tests.sh` 或等价脚本：将新 fixture 纳入 compat-runner 验证

## Capabilities

### 新增能力

- `prepared-stmt-protocol`：MySQL 风格预处理语句的完整生命周期管理（准备 → 执行 → 释放）
- `prepared-stmt-cache`：服务端prepared statement 缓存，支持通过名称复用
- `wire-comstmt-decoding`：网络层 COM_STMT_PREPARE / EXECUTE / CLOSE 包解码

### 修改能力

- `parser-prepare-expr`：现有 `PREPARE name FROM 'sql'` 语法扩展支持参数绑定（`?` 占位符）
- `executor-stmt-dispatch`：`Statement` 枚举增加 `Prepare` / `Execute` / `Deallocate` 分支到执行器的路由

## Impact

- **新增文件**：`crates/executor/src/prepared_stmt.rs`（缓存实现）、`tests/compat/prepared_stmt_protocol.sql` + `.out`
- **修改文件**：`crates/parser/src/parser.rs`（确认语法完整性）、`crates/executor/src/lib.rs` 或等价文件（路由）
- **风险**：预处理语句缓存若未限制最大数量，可能导致内存泄漏；需在 cache 实现中加入 LRU 淘汰策略
- **无新增外部 crate 依赖**
