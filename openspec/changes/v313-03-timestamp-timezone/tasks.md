# V313-03: TIMESTAMP WITH TIME ZONE 支持 — tasks

> **Status**: 🔵 OPEN — created 2026-08-09 (Asia/Shanghai)
> **Author**: openclaw
> **Source**: V312-21 §5.1 `timestamp_timezone_deferred`
> **Branch**: `develop/v3.13.0`
> **Expiry**: 2027-06-30（如果未完成则关闭 deferred 记录）
> **Acceptance**: `tests/compat/mysql_v3_12/timestamp_timezone_deferred.sql` PASS + `SURFACE_DISPOSITION.md` `TIMESTAMP` 行更新为 `PASS`

---

## Phase 1: 类型系统扩展

- [ ] 1.1 `crates/types/src/sql_type.rs`：添加 `TimeZoneMode` 枚举（`None`、`WithTimeZone`、`WithLocalTimeZone`）
- [ ] 1.2 `crates/types/src/sql_type.rs`：将 `ColumnType::Timestamp` 变体从无参数升级为携带 `TimeZoneMode`
- [ ] 1.3 `crates/types/src/value.rs`：添加 `TimestampValue` 结构体（`epoch_ns: i64`、`tz: Option<TzId>`）
- [ ] 1.4 `crates/types/src/value.rs`：添加 `TzId` 枚举（`Offset(FixedOffset)`、`Session`）
- [ ] 1.5 `crates/types/src/value.rs`：实现 `TimestampValue::parse_from_str`（支持 `2026-08-09 10:30:00+08:00` 格式）
- [ ] 1.6 `crates/types/src/value.rs`：实现 `convert_to_utc(ts: TimestampValue, tz: TzId) -> i64`
- [ ] 1.7 `crates/types/src/value.rs`：实现 `convert_from_utc(epoch_ns: i64, tz: TzId) -> i64`
- [ ] 1.8 更新所有 `ColumnType::Timestamp` 调用方（`crates/parser/src/ast.rs`、`crates/executor/src/**/*.rs`），适配新的 `Timestamp(TimeZoneMode)` 变体

## Phase 2: Parser 层

- [ ] 2.1 `crates/parser/src/ast.rs`：在 `ColumnTypeDef` 中添加 `tz_mode: TimeZoneMode` 字段
- [ ] 2.2 `crates/parser/src/parser.rs`：在 `parse_data_type()` 中识别 `TIMESTAMP [WITH TIME ZONE | WITH LOCAL TIME ZONE]` 语法
- [ ] 2.3 `crates/parser/src/parser.rs`：确保无后缀的 `TIMESTAMP` 解析为 `TimeZoneMode::WithLocalTimeZone`（MySQL 兼容默认）
- [ ] 2.4 `crates/parser/src/parser.rs`：解析 `TIMESTAMP(n)` 中的精度参数（n = 0~6）
- [ ] 2.5 添加 parser 单元测试：`tests/parser/timestamp_timezone_test.rs`

## Phase 3: Executor 层

- [ ] 3.1 `crates/executor/src/insert.rs`：在 `evaluate_insert_value` 中，当 `col_type` 为 `Timestamp(tz_mode)` 时执行时区转换
- [ ] 3.2 `crates/executor/src/insert.rs`：解析 `time_zone` 会话变量（支持 `'+08:00'`、`'SYSTEM'` 等格式）
- [ ] 3.3 `crates/executor/src/select.rs`：在 `eval_timestamp_projection` 中，当 `tz_mode != None` 时执行 UTC→会话时区逆转换
- [ ] 3.4 `crates/executor/src/session.rs`：确保每个会话有独立的 `time_zone` 变量，默认为 `'SYSTEM'` 或 `'+00:00'`
- [ ] 3.5 添加 executor 单元测试：`tests/executor/timestamp_timezone_test.rs`

## Phase 4: Wire 协议层（最小变更）

- [ ] 4.1 检查 `ColumnDefinition` wire 编码，确认 `MYSQL_TYPE_TIMESTAMP` 不需要修改
- [ ] 4.2 验证 `time_zone` 会话变量通过 `COM_QUERY` (`SET time_zone = '+08:00'`) 可以正确设置
- [ ] 4.3 验证带时区的 timestamp 值通过 wire 协议往返后值不变

## Phase 5: Compat Fixture 升级（V312-21 follow-up）

- [ ] 5.1 `tests/compat/mysql_v3_12/timestamp_timezone_deferred.sql`：更新 `expect:` 从 `DEFERRED:<issue-link>` 到 `PASS`
- [ ] 5.2 `tests/compat/mysql_v3_12/timestamp_timezone_deferred.out`：更新预期输出为时区转换后的结果
- [ ] 5.3 `tests/compat/mysql_v3_12/timestamp_timezone_deferred.sql`：扩展覆盖场景：
  - 不同时区会话（`'+00:00'`、`'+08:00'`、`'-05:00'`）
  - 裸 `TIMESTAMP` 与 `TIMESTAMP WITH TIME ZONE` 的差异
  - `TIMESTAMP WITH LOCAL TIME ZONE` 行为
- [ ] 5.4 运行 `bash scripts/gate/check_v312_21_mysql_compat.sh`，确认 `timestamp_timezone_deferred.sql` 返回 PASS
- [ ] 5.5 更新 `docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md`：`TIMESTAMP` 行从 `deferred` 更新为 `PASS`，添加 evidence_hash

## Phase 6: 文档和收尾

- [ ] 6.1 `docs/releases/v3.13.0/TIMESTAMP_TIMEZONE_IMPL.md`：记录实现决策、类型变更、测试结果
- [ ] 6.2 更新 `crates/types/src/sql_type.rs` 的 doc comment，记录 `TimeZoneMode` 用法
- [ ] 6.3 更新 `docs/releases/v3.13.0/RELEASE_NOTES.md` MySQL 兼容性章节：添加 `TIMESTAMP WITH TIME ZONE` 为 PASS
- [ ] 6.4 在 issue #3908（或其 v3.13.0 对应 issue）中评论：V313-03 完成，TIMESTAMP deferred 已关闭

---

## Estimate breakdown

| Phase | Description | Hours |
|-------|-------------|-------|
| 1 | 类型系统扩展 | 8h |
| 2 | Parser 层 | 6h |
| 3 | Executor 层 | 10h |
| 4 | Wire 协议层（验证） | 2h |
| 5 | Compat Fixture 升级 | 4h |
| 6 | 文档和收尾 | 2h |
| **Total** | | **32h** |

## Carried items

无 — 本变更覆盖 V312-21 §5.1 的全部 scope。
