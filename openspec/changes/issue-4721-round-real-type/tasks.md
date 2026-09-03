# Tasks — issue-4721 ROUND REAL type / Float display

## T1: Value::Float 渲染保真

- [x] T1.1 `Value::to_sql_string()` Float arm: 整数值有限浮点输出 `{:.1}`
- [x] T1.2 `|f| < 1e15` 上限保护（超大数保持科学计数/原格式）
- [x] T1.3 更新 value.rs `test_value_sql_string_float` 断言
- [x] T1.4 更新 trigger.rs `test_value_to_sql_literal_float` 断言

## T2: CLI 渲染统一

- [x] T2.1 `output.rs::value_to_string` Float arm 委托 `to_sql_string()`
- [x] T2.2 `json_value` 保持 `f.to_string()`（JSON 数字无类型差异）

## T3: ROUND 返回类型跟随输入

- [x] T3.1 `d <= 0` 且输入 Integer → `Value::Integer`（MySQL 语义）
- [x] T3.2 `d <= 0` 且输入非 Integer → `Value::Float`（SQLite 语义）
- [x] T3.3 `d > 0` 维持 `Value::Float`（#4613 不回归）

## T4: 验证

- [x] T4.1 实跑 6 条 SQL 全部符合期望（4.0/3.5/35.0/35.0/3.0/3）
- [x] T4.2 cargo test -p sqlrustgo-types PASS
- [x] T4.3 cargo test -p sqlrustgo-executor --lib 764 passed
- [x] T4.4 cargo test -p sqlrustgo --lib 115 passed
- [x] T4.5 cli_test 6 个失败确认为 pre-existing（develop 同样失败）
