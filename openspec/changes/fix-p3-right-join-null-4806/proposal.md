## Why

Issue #4806: `RIGHT JOIN` 在 sqlrustgo 中返回空字符串 `''` 而非
`NULL` 当 unmatched right-side 行存在时。

SQL 标准:任何 JOIN unmatched row 的 non-mapped 列必须是 NULL。
PostgreSQL/MySQL/SQLite 都遵循此规则。当前 bug 把 NULL 错误序列化为
空字符串,造成:
- 类型断言失败:`IS NULL` 检查漏判空字符串。
- 数据丢失:`COALESCE(col, 'default')` 返回空字符串而不是 default。
- 用户预期错位:报表统计(COUNT NULL)出错。

可能根因:
1. Hash join unmatched row 直接用 `Value::Text("")` 填充而非
   `Value::Null`。
2. Serialization 路径有 `.unwrap_or_default()` 把 None 替换为 String
   default (空字符串)而非保留 None。
3. Test harness / display formatter 把 NULL 显示为空字符串(只是
   display bug,实际存储是 NULL,验证通过 storage probe 即可区分)。

## What Changes

根据根因类型:

1. **如果是 serialization bug**: 修复 display / serialization 路径,
     把 `None` 正确显示为 `NULL` 而不是空字符串。
2. **如果是 join executor bug**: 在 engine_select.rs 的 hash-join
     unmatched-row 处理中,unmapped column 应该填 `Value::Null` 而
     不是 `Value::Text("")`。
3. **如果是类型转换 bug**: 修复 `Value::Option<T>` → SQL 序列化路
     径,保留 None → NULL。

## Capabilities

### New Capabilities

- `executor-right-join-null-serialization`: RIGHT JOIN unmatched right-
  side 行 MUST 在 left columns 填 NULL(非空字符串)。

## Out of Scope

- LEFT JOIN / FULL OUTER JOIN 序列化:同时验证(应在同一 fix 中
  验证一致性)。
- Anti-join (NOT IN / NOT EXISTS): 不相关。
- Display formatter 调整(只影响输出,不影响存储):本 PR 修复存储/
  executor 层,formatter 是 follow-up。

## Verification

- 新测试 `tests/integration/sql/p3_right_join_null_4806_test.rs`:
  - `right_join_unmatched_returns_null` — anchor:`SELECT l.*, r.* 
    FROM l RIGHT JOIN r ON l.id = r.id WHERE r.id = 2` 返回行,left
    columns 为 NULL (不是 '')。
  - `right_join_null_is_not_empty_string` — 验证 IS NULL 判断正确:
    `WHERE l.id IS NULL` 返回该行,而 `WHERE l.id = ''` 不返回。
  - `right_join_coalesce_returns_default` — `COALESCE(l.id, 'default')`
    返回 'default' (不是 '')。
  - `right_join_count_null_works` — `COUNT(l.id)` 不计 NULL 行。
  - `right_join_full_outer_null` — FULL OUTER JOIN 同样行为。
  - `right_join_left_columns_typed_null` — `l.id` column type 仍
    是 Integer,只是 value 是 NULL。

Total: 6 tests。

- `cargo test --test p3_right_join_null_4806_test` 6/6 PASS。
- 回归: full_outer_join_test (v312-95 PR #4789 引入), right_join_test,
  parser_e2e_test。

## Risks / Trade-offs

- **v312-95 已经部分修复**: PR #4789 (v312-95 #4639) 引入
  `test_full_outer_join_preserves_both_sides`,engine_select.rs:4408-
  4474 是 hash-join path。Issue #4806 报告时 PR #4789 可能还未 merged
  或 PR #4789 之后又有 regression。
- **Display 区分**: 即使 executor 正确产生 NULL,客户端 (REPL, 网络
  协议) 可能错误显示空字符串。验证:`SELECT * FROM l RIGHT JOIN r 
  ...` 后 client output 必须显式显示 "NULL" 字面量。
- **CSV/JSON export**: export 路径同样需要 NULL → NULL,不是空字符串。
  本 PR 范围限于 SQL 层,export 是后续。

## Initial Verification Step

```bash
# 验证 v312-95 PR #4789 是否仍生效
git log --oneline --all --grep "RIGHT JOIN\|full outer"
# 跑现有 full_outer_join_test
cargo test --test test_full_outer_join_preserves_both_sides 2>/dev/null \
  || cargo test --test <name_v312_95_used>
```

如果现有测试全绿,Issue #4806 极可能是 fixed-by-existing-code,只需
写新测试 pin 住行为,关闭 issue,无代码改动。

参考 memory: [[v312-95-4639-right-full-join-closure]] 已经记录
fixed-by-existing-code 状态。