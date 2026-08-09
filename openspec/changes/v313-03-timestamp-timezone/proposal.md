# V313-03: TIMESTAMP WITH TIME ZONE 支持 — proposal

> **Author**: openclaw
> **Date**: 2026-08-09 (Asia/Shanghai)
> **Branch**: `develop/v3.13.0`
> **Tracking issue**: （跟随 V312-21 遗留问题）
> **Source**: V312-21-mysql-compat-sql-surface-backlog §5.1 `timestamp_timezone_deferred`
> **Expiry**: 2027-06-30

## Why

V312-21 MySQL 兼容性清理（issue #3908）在 `SURFACE_DISPOSITION.md` 中将 `TIMESTAMP` 类型标记为 `deferred`：

```
| TIMESTAMP | v3.7-v3.10 claimed | no TZ handling | deferred | <hash> | openclaw | 2027-06-30 |
```

当前状态：`TIMESTAMP` 能存储但**不携带时区信息**，即 `CREATE TABLE t (ts TIMESTAMP)` 被接受，但 `INSERT`/`SELECT` 不会进行任何时区转换。MySQL 语义要求 `TIMESTAMP WITH TIME ZONE`（或 `TIMESTAMP` 声明后自动使用会话时区），而当前实现等价于"裸 UNIX epoch 整数"。

本变更在 v3.13.0 中正式实现 `TIMESTAMP WITH TIME ZONE` 语法和相关运行时行为，消除这一 deferred 记录。

## What Changes

1. **Parser 层**：识别 `TIMESTAMP` 与 `TIME ZONE` 的组合语法（`TIMESTAMP WITH TIME ZONE`、`TIMESTAMP WITH LOCAL TIME ZONE`），扩展 `ColumnTypeDef` AST 节点，携带时区标记。
2. **Type 系统层**：将 `ColumnType::Timestamp` 升级为带时区模式的枚举变体（`Timestamp(Option<TzId>)`），内部存储从 `i64`（epoch）扩展为可附加 `TzId` 的结构。
3. **Executor 层**：`INSERT` 时将裸时间戳值按会话时区（`time_zone` 会话变量）转换为 UTC 存储；`SELECT` 时从 UTC 按会话时区逆转换输出。
4. **Wire 层**：在 `ColumnDefinition` 包中携带时区信息（MySQL wire 协议 `MYSQL_TYPE_TIMESTAMP` 不直接编码时区，时区通过初始化连接时的 `time_zone` 变量控制）。
5. **Compat fixture**：V312-21 的 `tests/compat/mysql_v3_12/timestamp_timezone_deferred.sql` 升级为 PASS fixture，验证时区转换语义。

## Goals / Non-Goals

**Goals**：
- Parser 接受 `TIMESTAMP WITH TIME ZONE` 和 `TIMESTAMP WITH LOCAL TIME ZONE` 语法（MySQL 兼容）
- `ColumnTypeDef` 携带时区模式，`ColumnType::Timestamp` 变体携带 `TzId`
- `Value::Timestamp` 支持带时区的时间戳字面量解析（`2026-08-09 10:30:00+08:00`）
- `INSERT` 时按会话时区自动转换为 UTC；`SELECT` 时按会话时区从 UTC 逆转换
- Compat fixture PASS

**Non-Goals**：
- 实现 `AT TIME ZONE` 投影语法（PostgreSQL 风格）— 放在 v3.14+
- 实现 `TIMESTAMPTZ`（带时区字面量）作为独立类型别名 — 与 MySQL 兼容性冲突
- 时区数据库（Olson/tzdata）集成 — 当前使用固定偏移量或会话级 `time_zone` 变量
- `ALTER TABLE ... MODIFY COLUMN` 的时区类型迁移

## Capabilities

### New Capabilities

- `timestamp-with-timezone`：Parser 识别 `TIMESTAMP WITH TIME ZONE` 语法，`ColumnType` 变体携带时区元数据
- `timestamp-timezone-conversion`：Executor 在 INSERT/SELECT 时按会话 `time_zone` 变量进行时区转换

### Modified Capabilities

- `mysql-compat-fixture-suite`：V312-21 deferred fixture 升级为 PASS
- `SURFACE_DISPOSITION.md`：`TIMESTAMP` 行从 `deferred` 更新为 `PASS`

## Impact

- **New**：无新 crate
- **Modified**：`crates/types/src/value.rs`（`Value::Timestamp` 扩展），`crates/parser/src/ast.rs`（`ColumnTypeDef` 扩展），`crates/executor/src/insert.rs` / `crates/executor/src/select.rs`（时区转换逻辑）
- **No new deps**：使用现有的会话 `time_zone` 变量系统（无需引入 tzdata）

## Dependencies

- **V312-21**：`tests/compat/mysql_v3_12/timestamp_timezone_deferred.sql` fixture 存在，需要升级为 PASS
- **V313-01~02**：Parser 和 Type 系统基础变更（如还未合并，需协调）
