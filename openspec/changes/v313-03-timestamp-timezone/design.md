# V313-03: TIMESTAMP WITH TIME ZONE 支持 — design

## 概述

实现 MySQL 兼容的 `TIMESTAMP WITH TIME ZONE` 类型。核心设计原则：

1. **最小侵入**：在现有 `ColumnType::Timestamp` 基础上增加变体，不破坏现有二进制兼容
2. **会话时区驱动**：使用已有的 `time_zone` 会话变量，不引入外部 tzdata 依赖
3. **两阶段转换**：INSERT 时会话时区→UTC，SELECT 时 UTC→会话时区

## 类型系统设计

### ColumnType 扩展

```rust
// crates/types/src/sql_type.rs
pub enum ColumnType {
    // ... 其他变体 ...
    Timestamp(TimeZoneMode),  // 新增：替换现有的 Timestamp
}

pub enum TimeZoneMode {
    None,              // 裸 TIMESTAMP，无时区（向后兼容）
    WithTimeZone,      // TIMESTAMP WITH TIME ZONE
    WithLocalTimeZone, // TIMESTAMP WITH LOCAL TIME ZONE（会话时区）
}
```

**向后兼容策略**：`ColumnType::Timestamp(TimeZoneMode::None)` 在功能上等价于 v3.12 的 `ColumnType::Timestamp`（无参数版本）。无参数的 `TIMESTAMP` 列类型解析为 `TimeZoneMode::None`。

### Value 层扩展

```rust
// crates/types/src/value.rs
pub enum Value {
    // ...
    Timestamp(TimestampValue),  // 内部结构变更
}

pub struct TimestampValue {
    pub epoch_ns: i64,      // UTC epoch，纳秒精度
    pub tz: Option<TzId>,   // 来源时区（用于字面量解析）
}

pub enum TzId {
    Offset(chrono::FixedOffset),  // 固定偏移量 +08:00
    Session,                       // 来自会话 time_zone 变量
}
```

### 字面量解析

`2026-08-09 10:30:00+08:00` → `TimestampValue { epoch_ns: ..., tz: Some(TzId::Offset(+08:00)) }`

转换步骤：
1. 解析器识别 `TIMESTAMP WITH TIME ZONE` 语法，注入 `TimeZoneMode::WithTimeZone`
2. Lexer 识别时区后缀（`+HH:MM`、`-HH:MM`、`UTC` 等），转换为 `TzId::Offset`
3. `TimestampValue::parse_from_str` 执行"字面量时间 + 时区 → UTC epoch" 转换

## Parser 设计

### 语法扩展

```sql
column_type ::=
    TIMESTAMP [ (precision) ] [ WITH TIME ZONE | WITH LOCAL TIME ZONE ]

-- MySQL 兼容：TIMESTAMP 等价于 TIMESTAMP WITH LOCAL TIME ZONE
-- （默认使用会话时区）
```

### AST 变更

```rust
// crates/parser/src/ast.rs

pub struct ColumnTypeDef {
    pub data_type: DataType,
    pub length: Option<Length>,
    pub tz_mode: TimeZoneMode,  // 新增字段
}

pub enum TimeZoneMode {
    None,
    WithTimeZone,
    WithLocalTimeZone,
}
```

Parser 修改点：
- `parse_data_type()`：识别 `WITH TIME ZONE` / `WITH LOCAL TIME ZONE` 关键字序列
- `parse_create_table_column()`：将 `tz_mode` 传递给 `ColumnTypeDef`
- 不需要修改 grammar file（使用 `token_predicate` 即可覆盖 `TIMESTAMP` 系列关键字）

## Executor 设计

### INSERT 时区转换

```rust
fn evaluate_insert_value(
    value: &Value,
    col_type: &ColumnType,
    session: &Session,
) -> Value {
    match (value, col_type) {
        (Value::Timestamp(ts_val), ColumnType::Timestamp(tz_mode)) => {
            let tz = resolve_tz(tz_mode, session);
            let utc_epoch = convert_to_utc(ts_val, tz);
            Value::Timestamp(TimestampValue { epoch_ns: utc_epoch, tz: None })
        }
        _ => value.clone(),
    }
}
```

### SELECT 时区逆转换

```rust
fn eval_timestampProjection(
    epoch_ns: i64,
    tz_mode: TimeZoneMode,
    session: &Session,
) -> Value {
    let tz = resolve_tz(tz_mode, session);
    let local = convert_from_utc(epoch_ns, tz);
    Value::Timestamp(TimestampValue { epoch_ns: local, tz: Some(tz) })
}
```

### 会话时区解析

`time_zone` 会话变量格式（与 MySQL 兼容）：
- `'+08:00'` / `'-05:00'`：固定偏移量
- `'UTC'` / `'America/New_York'`：命名时区（仅支持固定偏移量子集）
- `'SYSTEM'`：使用 server 主时区

## Wire 协议层

MySQL wire `ColumnDefinition` 中 `MYSQL_TYPE_TIMESTAMP` 不直接编码时区信息。时区通过以下机制传递：

1. **连接初始化**：客户端发送 `SET time_zone = '+08:00'`
2. **Server 端时区变量**：每个会话持有独立的 `time_zone` 变量
3. **Timestamp 值编码**：仍然使用 `MYSQL_TYPE_TIMESTAMP` 的 4-byte 或 7-byte 格式（无时区），时区转换在应用层完成

**不修改 wire 协议**。现有 `ColumnDefinition` 的 `MYSQL_TYPE_TIMESTAMP` 足以覆盖。

## Dialect差异

| 方言 | TIMESTAMP 语义 | WITH TIME ZONE |
|------|----------------|----------------|
| MySQL | TIMESTAMP = TIMESTAMP WITH LOCAL TIME ZONE | 支持 |
| PostgreSQL | TIMESTAMP = TIMESTAMP WITHOUT TIME ZONE | 不支持（用 TIMESTAMPTZ） |
| SQL Server | DATETIME2 = 无时区 | 不支持 |

本设计兼容 MySQL 语义。PostgreSQL `TIMESTAMPTZ` 字面量（`'2026-08-09 10:30:00+08:00'::timestamptz`）在 PostgreSQL 方言下按各自方言处理，不在本变更范围内。

## 关键决策

### 决策 1：TIMESTAMP 默认时区模式

**选择**：`TIMESTAMP` 无后缀时解析为 `TimeZoneMode::WithLocalTimeZone`（会话时区）。`TIMESTAMP WITH TIME ZONE` 显式使用字面量时区。

**原因**：MySQL 默认行为。`CREATE TABLE t (ts TIMESTAMP)` 等价于 `TIMESTAMP WITH LOCAL TIME ZONE`，值在存储时转换为 UTC，查询时按会话时区显示。

**备选**：`TIMESTAMP` 解析为 `TimeZoneMode::None`（无时区）—— 被拒绝，因为与 MySQL 语义不符。

### 决策 2：时区数据库

**选择**：不引入 tzdata/Olson 时区数据库，仅支持固定偏移量（`+HH:MM`）。

**原因**：v3.13.0 计划中 tzdata 集成是独立大项（涉及存储体积、网络下载、IANA DB 更新等）。本变更聚焦于核心时区转换语义，不引入新依赖。

**未来扩展**：固定偏移量支持命名时区（`'America/New_York'` → 查找缓存的 `FixedOffset`）。

### 决策 3：Timestamp 精度

**选择**：内部使用纳秒精度（`i64` epoch_ns），与现有 `Value::Timestamp` 二进制编码兼容。

**原因**：`Value::Timestamp` 的现有字节编码为 `epoch_i64 + nano_i32`。该编码在网络层（B+Tree 页面、wire 协议）已广泛使用。扩展时区模式不改变编码格式——`epoch_ns` 保持不变，时区信息存储在 `ColumnType` 元数据中。

## 测试策略

### 单元测试

- `crates/types/src/value.rs`：测试 `TimestampValue::parse_from_str`（固定偏移量解析）
- `crates/parser/src/parser.rs`：测试 `TIMESTAMP WITH TIME ZONE` 语法解析
- `crates/executor/src/insert.rs`：测试 INSERT 时区转换
- `crates/executor/src/select.rs`：测试 SELECT 时区逆转换

### Compat Fixture（V312-21 升级）

```sql
-- tests/compat/mysql_v3_12/timestamp_timezone_deferred.sql
# name: TIMESTAMP WITH TIME ZONE
# expect: PASS
SET time_zone = '+08:00';
CREATE TABLE t (ts TIMESTAMP WITH TIME ZONE);
INSERT INTO t VALUES ('2026-08-09 10:30:00+08:00');
SELECT ts FROM t;
DROP TABLE t;
```

预期输出：查询返回 `'2026-08-09 10:30:00+08:00'`（INSERT 时区被保留）。

### 兼容性测试

- `TIMESTAMP WITH TIME ZONE` 列上 `SELECT` 不同时区会话的输出差异
- 裸 `TIMESTAMP`（无 `WITH TIME ZONE`）按 MySQL 默认行为处理
- `TIMESTAMP WITH LOCAL TIME ZONE` 在不同 `time_zone` 会话变量下的行为

## 迁移计划

| 阶段 | 风险 | 回滚 |
|------|------|------|
| Parser `TimeZoneMode` 字段添加 | 低 — 新字段，默认 `None` | git revert |
| Type 系统升级 `ColumnType::Timestamp` | 中 — 调用方需要适配 | git revert |
| Executor 时区转换逻辑 | 中 — 语义正确性 | 暂时禁用时区转换 |
| Compat fixture PASS | 低 — fixture 自验证 | 降级为 deferred |

## 验证计划

1. **Parser 验证**：`cargo test -p sqlrustgo-parser timestamp_timezone` — 确认语法解析正确
2. **Unit 验证**：`cargo test -p sqlrustgo-types timestamp` — 确认时区解析/转换正确
3. **Compat runner**：`bash scripts/gate/check_v312_21_mysql_compat.sh` — `timestamp_timezone_deferred.sql` 升级为 PASS
4. **向后兼容**：`cargo test --workspace` — 现有 `TIMESTAMP` 列行为不变
5. **Clippy + Fmt**：通过 P1/P2 门禁
