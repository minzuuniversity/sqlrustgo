# Issue #4721: ROUND(real, int) 丢失 REAL 类型 / 整数值 Float 渲染丢 `.0`

## Why

回归验证（2026-09-03, HEAD `11c0ac08d1`）确认 #4721 仍存在：

```sql
SELECT round(3.5, 0);        -- 实: 4   (Integer) — 期望 4.0 (REAL)
SELECT round(70.0 * 0.5, 2); -- 实: 35  (Integer) — 期望 35.0 (REAL)
SELECT 35.0;                 -- 实: 35            — 期望 35.0
SELECT 1.5 + 1.5;            -- 实: 3             — 期望 3.0
```

两个根因：

1. **渲染层**: CLI `value_to_string()` 与 `Value::to_sql_string()` 的
   `Value::Float` arm 都用 `f.to_string()`，Rust 对整数值 f64 输出
   `"35"` 而非 `"35.0"`。`parse_lit` 的 #4610 Float 保真修复因此对
   用户不可见（值对了，显示还是整数形态）。
2. **函数层**: `eval_fn("ROUND")` 的 `d <= 0` 分支无条件
   `Value::Integer(rounded as i64)`，把 Float 输入（`round(3.5, 0)`）
   强转成 Integer，REAL 类型在函数边界被吃掉。

影响：TPC-H/TPC-DS REAL 表达式与聚合输出全部呈整数形态；SQLite 兼容
测试（`round()` 恒返 REAL）失败。

## What Changes

### Value::Float 渲染（crates/types/src/value.rs, ~10 行）

- `to_sql_string()` 的 Float arm：整数值、有限、|f| < 1e15 时输出
  `format!("{:.1}", f)`（`"35.0"`、`"0.0"`、`"-4.0"`）；其余保持
  最短 round-trip 格式（`"3.14"`、`"1e20"`、`"NaN"`）。
- `impl Display for Value` 委托 `to_sql_string()`，自动获得同一行为。

### CLI 渲染统一（crates/sqlrustgo-cli/src/output.rs, ~5 行）

- `value_to_string()` 的 Float arm 从重复的 `f.to_string()` 改为
  委托 `Value::to_sql_string()`（types 层是唯一实现点）。
- JSON 输出 (`json_value`) 保持 `f.to_string()`——JSON 数字 35 与
  35.0 无类型语义差异，避免破坏 JSON 消费者。

### ROUND 返回类型跟随输入（crates/executor/src/expr/mod.rs, ~10 行）

- `d <= 0` 分支：输入为 `Value::Integer` → `Value::Integer`（MySQL:
  `ROUND(3)` → `3`）；否则 → `Value::Float`（SQLite: `round(3.5,0)`
  → `4.0`）。
- `d > 0` 分支维持 `Value::Float`（#4613 行为不变）。

### 测试

- value.rs: `test_value_sql_string_float` 更新 + 新增 35.0/-4.0/3.14 断言。
- trigger.rs: `test_value_to_sql_literal_float` 的 `Float(0.0)=="0"`
  断言更新为 `"0.0"`（预期行为变化）。
- expr/mod.rs: ROUND 单测补 `round(3.5,0)=Float(4.0)`、`round(3,0)=Integer(3)`。

## Impact

- **破坏面**: 所有整数值 Float 的文本渲染从 `35` 变为 `35.0`
  （SQLite / MySQL 8.0 / PostgreSQL 均为 `35.0`，属兼容性修正）。
  已知 pre-existing cli_test 6 个失败与此无关（develop 上同样失败，
  系 CLI 参数解析回归，另行跟踪）。
- **不涉及**: 存储层编码、协议层（mysql-server 有自己的 Value 编码）、
  JSON 序列化。

## Verification

```sql
SELECT round(3.5, 0);        -- 4.0
SELECT round(3.5, 1);        -- 3.5
SELECT round(70.0 * 0.5, 2); -- 35.0
SELECT 35.0;                 -- 35.0
SELECT 1.5 + 1.5;            -- 3.0
SELECT round(3, 0);          -- 3    (Integer 输入保持 MySQL 语义)
```

- cargo test -p sqlrustgo-types: PASS
- cargo test -p sqlrustgo-executor --lib: 764 passed
- cargo test -p sqlrustgo --lib: 115 passed
